use serde_json::{Value, json};

use crate::keel::case::{course, ensure, expect, rows};
use crate::keel::probe::Client;
use crate::keel::site::Site;

const CLOSED: [&str; 3] = ["expires_at", "created_at", "updated_at"];

fn bare(row: &Value, what: &str) -> Result<(), String> {
    let found: Vec<&str> = CLOSED
        .into_iter()
        .filter(|field| row.get(field).is_some())
        .collect();
    ensure(
        found.is_empty(),
        &format!("{what} answers {found:?}: {row}"),
    )
}

async fn engine(sudo: &Client<'_>, unit: &str) -> Result<(), String> {
    let replies = [
        sudo.get(&format!("/{unit}")).await?,
        sudo.get(&format!("/{unit}/1")).await?,
        sudo.query(&format!("from {unit}")).await?,
        sudo.batch(json!([{ "verb": "end", "unit": unit, "id": 1 }]))
            .await?,
    ];
    for (reply, path) in replies.iter().zip(["list", "read", "query", "batch"]) {
        expect(reply.status, 404, &format!("{unit} {path}"))?;
        ensure(
            !reply.body.contains("hash"),
            &format!("{unit} {path} names a hash"),
        )?;
    }
    Ok(())
}

impl Site<'_> {
    pub async fn closed(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let made = sudo.post("/course", course("K1", "kept")).await?;
        expect(made.status, 201, "create course")?;
        bare(&made.json()?, "a created course")?;
        let id = made.id()?;
        let ada = sudo
            .post("/student", json!({ "no": "S1", "name": "ada" }))
            .await?;
        expect(ada.status, 201, "create student")?;
        let path = format!("/student/{}/courses", ada.id()?);
        let tie = sudo
            .post(&path, json!({ "right": id, "grade": "a" }))
            .await?;
        expect(tie.status, 201, "tie")?;
        bare(
            &sudo.get(&format!("/course/{id}")).await?.json()?,
            "a course",
        )?;
        for row in rows(&sudo.get("/course").await?)? {
            bare(&row, "a listed course")?;
        }
        let pack = sudo
            .query(r#"from Student where no = "S1" link courses"#)
            .await?;
        expect(pack.status, 200, "linked query")?;
        let pack = pack.json()?;
        bare(&pack["bags"]["student"][0], "a queried student")?;
        bare(&pack["bags"]["student.courses"][0], "a queried tie")?;
        engine(&sudo, "@seal").await?;
        engine(&sudo, "@pulse").await?;
        expect(sudo.get("/@grant").await?.status, 200, "grant list")?;
        let anon = self.client(None);
        expect(
            anon.get("/-/keel/pulse").await?.status,
            403,
            "anonymous pulse",
        )?;
        let trace = anon.get(&format!("/-/keel/row/course/{id}")).await?;
        expect(trace.status, 403, "anonymous row metadata")
    }
}
