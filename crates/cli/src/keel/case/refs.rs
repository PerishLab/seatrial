use serde_json::json;

use crate::keel::case::race::{live, told};
use crate::keel::case::{course, ensure, expect, rows};
use crate::keel::site::Site;

impl Site<'_> {
    pub async fn dangle(&self) -> Result<(), String> {
        self.tied(true).await
    }

    pub async fn cling(&self) -> Result<(), String> {
        self.tied(false).await
    }

    pub async fn orphan(&self) -> Result<(), String> {
        self.rooted(true).await
    }

    pub async fn adopt(&self) -> Result<(), String> {
        self.rooted(false).await
    }

    async fn tied(&self, early: bool) -> Result<(), String> {
        let (near, far) = self.pair().await?;
        let made = near.post("/course", course("D1", "target")).await?;
        expect(made.status, 201, "create course")?;
        let algo = made.id()?;
        let ada = near
            .post("/student", json!({ "no": "S01", "name": "ada" }))
            .await?;
        expect(ada.status, 201, "create student")?;
        let ada = ada.id()?;
        let end = format!("/course/{algo}");
        let tie = format!("/student/{ada}/courses");
        let body = json!({ "right": algo, "grade": "A" });
        let (first, second) = match early {
            true => self.race(near.delete(&end), far.post(&tie, body)).await?,
            false => self.race(near.post(&tie, body), far.delete(&end)).await?,
        };
        let ended = near.get(&end).await?.status == 404;
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

    async fn rooted(&self, early: bool) -> Result<(), String> {
        let (near, far) = self.pair().await?;
        let book = near.post("/ledger", json!({ "tag": "O1" })).await?;
        expect(book.status, 201, "create ledger")?;
        let book = book.id()?;
        let end = format!("/ledger/{book}");
        let body = json!({ "note": "late", "ledger": book });
        let (first, second) = match early {
            true => {
                self.race(near.delete(&end), far.post("/entry", body))
                    .await?
            }
            false => {
                self.race(near.post("/entry", body), far.delete(&end))
                    .await?
            }
        };
        let ended = near.get(&end).await?.status == 404;
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
