use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::keel::case::{course, ensure, expect};
use crate::keel::probe::Client;
use crate::keel::site::Site;

async fn count(sudo: &Client<'_>, text: &str) -> Result<Value, String> {
    let reply = sudo.query(text).await?;
    expect(reply.status, 200, text)?;
    Ok(reply.json()?["count"].clone())
}

impl Site<'_> {
    pub async fn ghost(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let made = sudo.post("/course", course("H1", "ghost")).await?;
        expect(made.status, 201, "create course")?;
        let algo = made.id()?;
        let ada = sudo
            .post("/student", json!({ "no": "S01", "name": "ada" }))
            .await?;
        expect(ada.status, 201, "create student")?;
        let ada = ada.id()?;
        let tie = sudo
            .post(
                &format!("/student/{ada}/courses"),
                json!({ "right": algo, "grade": "A" }),
            )
            .await?;
        expect(tie.status, 201, "tie")?;
        let gone = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("clock: {error}"))?
            .as_secs()
            - 10;
        self.sql(&format!(
            "UPDATE course SET expires_at = {gone} WHERE id = {algo}"
        ))
        .await?;
        let read = sudo.get(&format!("/course/{algo}")).await?;
        expect(read.status, 404, "read of the planted ended course")?;
        let linked =
            sudo.query("from Student link courses").await?.json()?["bags"]["student.courses"]
                .as_array()
                .map_or(0, Vec::len);
        let paths = [
            (
                "has",
                format!(r#"from Student where courses has "{algo}" count"#),
            ),
            (
                "some id",
                format!(r#"from Student where courses some(id = "{algo}") count"#),
            ),
            (
                "some grade",
                r#"from Student where courses some(grade = "A") count"#.to_string(),
            ),
            (
                "some code",
                r#"from Student where courses some(code = "H1") count"#.to_string(),
            ),
        ];
        let mut seen = vec![format!("link {linked}")];
        for (name, text) in &paths {
            seen.push(format!("{name} {}", count(&sudo, text).await?));
        }
        let told = seen.join(", ");
        println!("ghost   tie to an ended course: {told}");
        ensure(
            seen.iter().all(|path| path.ends_with(" 0")),
            &format!("read paths still see a tie to ended course {algo}: {told}"),
        )
    }
}
