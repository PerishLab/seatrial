use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::json;

use crate::keel::case::{course, ensure, expect, rows};
use crate::keel::probe::Client;
use crate::keel::site::Site;

async fn lease(sudo: &Client<'_>, unit: &str, id: &serde_json::Value) -> Result<(), String> {
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("clock: {error}"))?
        .as_secs() as i64
        + 3;
    let deed = json!([{ "verb": "end", "unit": unit, "id": id, "at": at }]);
    expect(sudo.batch(deed).await?.status, 200, "lease")
}

async fn pass() {
    tokio::time::sleep(Duration::from_secs(5)).await;
}

impl Site<'_> {
    pub async fn outlive(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let book = sudo.post("/ledger", json!({ "tag": "E1" })).await?;
        expect(book.status, 201, "create ledger")?;
        let book = book.id()?;
        lease(&sudo, "Ledger", &json!(book)).await?;
        let late = sudo
            .post("/entry", json!({ "note": "late", "ledger": book }))
            .await?
            .status;
        pass().await;
        expect(
            sudo.get(&format!("/ledger/{book}")).await?.status,
            404,
            "read after the lease",
        )?;
        let left = rows(&sudo.get("/entry").await?)?;
        println!(
            "lease   outlive: entry under a leased ledger answers {late}, {} live after",
            left.len()
        );
        ensure(
            left.is_empty(),
            &format!("{} entries outlived their leased ledger {book}", left.len()),
        )
    }

    pub async fn linger(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let made = sudo.post("/course", course("G1", "leased")).await?;
        expect(made.status, 201, "create course")?;
        let algo = made.id()?;
        let ada = sudo
            .post("/student", json!({ "no": "S01", "name": "ada" }))
            .await?;
        expect(ada.status, 201, "create student")?;
        let ada = ada.id()?;
        lease(&sudo, "Course", &json!(algo)).await?;
        let late = sudo
            .post(
                &format!("/student/{ada}/courses"),
                json!({ "right": algo, "grade": "A" }),
            )
            .await?
            .status;
        pass().await;
        expect(
            sudo.get(&format!("/course/{algo}")).await?.status,
            404,
            "read after the lease",
        )?;
        let count = sudo
            .query(&format!(r#"from Student where courses has "{algo}" count"#))
            .await?
            .json()?["count"]
            .clone();
        println!("lease   linger: tie to a leased course answers {late}, count {count} after");
        ensure(
            count == 0,
            &format!("{count} students still take leased course {algo}"),
        )
    }
}
