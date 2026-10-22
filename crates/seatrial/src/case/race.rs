use std::future::Future;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::case::{course, ensure, expect, rows};
use crate::probe::{Client, Reply};
use crate::site::Site;

const HOLDER: &str = "seatrial-hold";
const SEEN: &str = "SELECT string_agg(concat_ws(':', application_name, wait_event_type, wait_event, state, left(regexp_replace(query, '\\s+', ' ', 'g'), 90)), ' | ') FROM pg_stat_activity WHERE datname = current_database()";
const WAIT: Duration = Duration::from_secs(8);

fn live(list: &[Value], field: &str, value: &Value) -> usize {
    list.iter().filter(|row| &row[field] == value).count()
}

fn told(first: &Reply, second: &Reply) -> String {
    format!("answers {} then {}", first.status, second.status)
}

impl Site<'_> {
    async fn pair(&self) -> Result<(Client<'_>, Client<'_>), String> {
        let auth = self.sealed().await?;
        let near = self.client(Some(auth.clone()));
        let seed = near.post("/course", course("SEED", "clock")).await?;
        expect(seed.status, 201, "seed write")?;
        let far = self.twin(auth).await?;
        Ok((near, far))
    }

    async fn count(&self, text: &str) -> Result<usize, String> {
        let got = self.sql(text).await?;
        got.trim().parse().map_err(|_| format!("count {got:?}"))
    }

    async fn until(&self, text: &str, want: usize) -> Result<(), String> {
        let start = Instant::now();
        while start.elapsed() < WAIT {
            if self.count(text).await? >= want {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(format!("never reached {want}: {text}"))
    }

    async fn waiting(&self, want: usize) -> Result<(), String> {
        let text = "SELECT count(*) FROM pg_stat_activity WHERE datname = current_database() AND wait_event_type = 'Lock'";
        self.until(text, want).await
    }

    async fn hold(&self) -> Result<(), String> {
        let sql = "BEGIN; SELECT value FROM \"@clock\" WHERE name = 'pulse' FOR UPDATE; SELECT pg_sleep(30); COMMIT;";
        self.session(HOLDER, sql).await
    }

    async fn race<A, B>(&self, first: A, second: B) -> Result<(Reply, Reply), String>
    where
        A: Future<Output = Result<Reply, String>>,
        B: Future<Output = Result<Reply, String>>,
    {
        let held = format!(
            "SELECT count(*) FROM pg_stat_activity WHERE datname = current_database() AND application_name = '{HOLDER}' AND wait_event = 'PgSleep'"
        );
        let release = format!(
            "SELECT count(pg_terminate_backend(pid)) FROM pg_stat_activity WHERE datname = current_database() AND application_name = '{HOLDER}'"
        );
        let lead = async {
            self.until(&held, 1).await?;
            first.await
        };
        let tail = async {
            self.until(&held, 1).await?;
            self.waiting(1).await?;
            second.await
        };
        let open = async {
            self.until(&held, 1).await?;
            let waited = self.waiting(2).await;
            let seen = match &waited {
                Ok(()) => String::new(),
                Err(_) => self.sql(SEEN).await?,
            };
            self.sql(&release).await?;
            waited.map_err(|error| format!("{error}; sessions {seen}"))
        };
        let gate = async { tokio::join!(self.hold(), open).1 };
        let (lead, tail, open) = tokio::join!(lead, tail, gate);
        let (lead, tail) = (lead?, tail?);
        open.map_err(|error| format!("{error}; {}", told(&lead, &tail)))?;
        println!("race    {}: {}", self.case(), told(&lead, &tail));
        Ok((lead, tail))
    }

    pub async fn rival(&self) -> Result<(), String> {
        let (near, far) = self.pair().await?;
        let (first, second) = self
            .race(
                near.post("/course", course("X1", "near")),
                far.post("/course", course("X1", "far")),
            )
            .await?;
        let list = rows(&near.get("/course").await?)?;
        let held = live(&list, "code", &json!("X1"));
        ensure(
            held <= 1,
            &format!("{held} live courses hold X1; {}", told(&first, &second)),
        )
    }

    pub async fn lapse(&self) -> Result<(), String> {
        let (near, far) = self.pair().await?;
        let made = near.post("/course", course("Y1", "leased")).await?;
        expect(made.status, 201, "create leased course")?;
        let id = made.id()?;
        let until = json!([{ "verb": "end", "unit": "Course", "id": id, "at": 4_102_444_800_i64 }]);
        expect(near.batch(until).await?.status, 200, "lease")?;
        let (first, second) = self
            .race(
                near.post("/course", course("X2", "near")),
                far.patch(&format!("/course/{id}"), json!({ "code": "X2" })),
            )
            .await?;
        let list = rows(&near.get("/course").await?)?;
        let held = live(&list, "code", &json!("X2"));
        ensure(
            held <= 1,
            &format!("{held} live courses hold X2; {}", told(&first, &second)),
        )
    }

    pub async fn dangle(&self) -> Result<(), String> {
        let (near, far) = self.pair().await?;
        let made = near.post("/course", course("D1", "target")).await?;
        expect(made.status, 201, "create course")?;
        let algo = made.id()?;
        let ada = near
            .post("/student", json!({ "no": "S01", "name": "ada" }))
            .await?;
        expect(ada.status, 201, "create student")?;
        let ada = ada.id()?;
        let (first, second) = self
            .race(
                near.delete(&format!("/course/{algo}")),
                far.post(
                    &format!("/student/{ada}/courses"),
                    json!({ "right": algo, "grade": "A" }),
                ),
            )
            .await?;
        let ended = near.get(&format!("/course/{algo}")).await?.status == 404;
        let count = near
            .query(&format!(r#"from Student where courses has "{algo}" count"#))
            .await?
            .json()?["count"]
            .clone();
        ensure(
            !(ended && count != 0),
            &format!(
                "{count} students still take ended course {algo}; {}",
                told(&first, &second)
            ),
        )
    }

    pub async fn orphan(&self) -> Result<(), String> {
        let (near, far) = self.pair().await?;
        let book = near.post("/ledger", json!({ "tag": "O1" })).await?;
        expect(book.status, 201, "create ledger")?;
        let book = book.id()?;
        let (first, second) = self
            .race(
                near.delete(&format!("/ledger/{book}")),
                far.post("/entry", json!({ "note": "late", "ledger": book })),
            )
            .await?;
        let ended = near.get(&format!("/ledger/{book}")).await?.status == 404;
        let entries = rows(&near.get("/entry").await?)?;
        let held = live(&entries, "ledger", &json!(book));
        ensure(
            !(ended && held > 0),
            &format!(
                "{held} live entries under ended ledger {book}; {}",
                told(&first, &second)
            ),
        )
    }
}
