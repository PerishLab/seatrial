use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::case::{course, ensure, expect, rows};
use crate::probe::Reply;
use crate::site::{Site, Store};
use seatrial_lib::docker::{must, words};

impl Site<'_> {
    pub async fn foreign(&self) -> Result<(), String> {
        let auth = self.sealed().await?;
        let near = self.client(Some(auth.clone()));
        let made = near.post("/course", course("CS101", "near")).await?;
        expect(made.status, 201, "near create")?;
        let algo = made.id()?;
        for _ in 0..2 {
            let warm = rows(&near.get("/course").await?)?;
            ensure(warm.len() == 1, "near list before the far writes")?;
        }
        let far = self.twin(auth).await?;
        expect(
            far.post("/course", course("CS102", "far")).await?.status,
            201,
            "far create",
        )?;
        expect(
            far.delete(&format!("/course/{algo}")).await?.status,
            204,
            "far end",
        )?;
        let after = rows(&near.get("/course").await?)?;
        let ids: Vec<Value> = after.iter().map(|row| row["id"].clone()).collect();
        ensure(
            after.len() == 1 && after[0]["id"] != algo,
            &format!("near list after the far writes holds ids {ids:?}"),
        )
    }

    pub async fn revived(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        expect(
            sudo.post("/course", course("R1", "before")).await?.status,
            201,
            "create before",
        )?;
        if let Store::Pg = self.store() {
            let ended = self
                .sql("SELECT count(pg_terminate_backend(pid)) FROM pg_stat_activity WHERE datname = current_database() AND pid <> pg_backend_pid()")
                .await?;
            ensure(ended != "0", "no backend was terminated")?;
        }
        let mut failed = 0;
        let list = loop {
            let reply = sudo.get("/course").await?;
            if reply.status == 200 {
                break rows(&reply)?;
            }
            failed += 1;
            ensure(
                failed < 2,
                &format!(
                    "reads failed twice after the backend ended: {}",
                    reply.status
                ),
            )?;
        };
        ensure(list.len() == 1, "the row written before was lost")?;
        expect(
            sudo.post("/course", course("R2", "after")).await?.status,
            201,
            "create after",
        )
    }

    pub async fn vetoed(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let (find, plant) = match self.store() {
            Store::Pg => (
                "SELECT tablename FROM pg_tables WHERE schemaname = current_schema() AND tablename LIKE '%pulse%'",
                "ALTER TABLE \"{seat}\" ADD CONSTRAINT veto CHECK (false) NOT VALID",
            ),
            Store::Lite => (
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name LIKE '%pulse%'",
                "CREATE TRIGGER veto BEFORE INSERT ON \"{seat}\" BEGIN SELECT RAISE(ABORT, 'veto'); END",
            ),
        };
        let seat = self.sql(find).await?;
        ensure(
            !seat.is_empty() && !seat.contains('\n'),
            &format!("pulse tables: {seat:?}"),
        )?;
        self.sql(&plant.replace("{seat}", &seat)).await?;
        let put = sudo.post("/course", course("V1", "vetoed")).await?;
        ensure(
            put.status >= 400,
            &format!("write without its pulse answered {}", put.status),
        )?;
        let list = sudo.get("/course").await?;
        expect(list.status, 200, "list after the veto")?;
        ensure(rows(&list)?.is_empty(), "write committed without its pulse")
    }
}

const WAIT: Duration = Duration::from_secs(20);

fn down(reply: &Reply, what: &str) -> Result<(), String> {
    expect(reply.status, 503, what)?;
    ensure(
        reply
            .json()
            .is_ok_and(|body| body == json!({ "error": "store unavailable" })),
        &format!("{what} answers {}", reply.body),
    )
}

impl Site<'_> {
    async fn gate(&self, open: bool) -> Result<(), String> {
        let db = format!("trial_{}", self.case());
        let host = self.run().lane.host(crate::pg::NAME);
        let alter = format!("ALTER DATABASE {db} ALLOW_CONNECTIONS {open}");
        let cut = format!(
            "SELECT count(pg_terminate_backend(pid)) FROM pg_stat_activity WHERE datname = '{db}'"
        );
        for sql in [alter, cut] {
            let args = [
                "exec",
                &host,
                "psql",
                "-U",
                "postgres",
                "-h",
                "127.0.0.1",
                "-d",
                "postgres",
                "-tAc",
                &sql,
            ];
            must(&words(&args)).await?;
        }
        Ok(())
    }

    pub async fn outage(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        expect(
            sudo.post("/course", course("O1", "kept")).await?.status,
            201,
            "create course",
        )?;
        self.gate(false).await?;
        let first = sudo.get("/course").await?;
        let second = sudo.get("/course").await?;
        let health = sudo.get("/health").await?;
        println!(
            "outage  reads answer {} then {}, health {}",
            first.status, second.status, health.status
        );
        down(&first, "a read as the store drops")?;
        down(&second, "a read while the store refuses connections")?;
        down(&health, "health while the store refuses connections")?;
        self.gate(true).await?;
        let start = Instant::now();
        while start.elapsed() < WAIT {
            if sudo.get("/health").await?.status == 200 {
                let list = sudo.get("/course").await?;
                expect(list.status, 200, "a read once the store is back")?;
                return ensure(list.body.contains("\"O1\""), "the course was lost");
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Err("health never recovered once the store was back".into())
    }
}
