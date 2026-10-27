use serde_json::{Value, json};

use crate::keel::case::{course, ensure, expect, rows};
use crate::keel::probe::Client;
use crate::keel::site::Site;

async fn grade(sudo: &Client<'_>, want: &str) -> Result<(), String> {
    let linked = sudo
        .query(r#"from Student where no = "S01" link courses"#)
        .await?;
    expect(linked.status, 200, "linked query")?;
    let pack = linked.json()?;
    let students = pack["bags"]["student"].as_array().map_or(0, Vec::len);
    ensure(
        students == 1,
        &format!("linked query found {students} students"),
    )?;
    let ties = pack["bags"]["student.courses"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    ensure(
        ties.len() == 1,
        &format!("linked query found {} ties", ties.len()),
    )?;
    ensure(
        ties[0]["grade"] == want,
        &format!("tie grade is {}, wanted {want}", ties[0]["grade"]),
    )
}

impl Site<'_> {
    pub(crate) async fn admin(&self) -> Result<Client<'_>, String> {
        let auth = self.sealed().await?;
        Ok(self.client(Some(auth)))
    }

    pub async fn portable(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let made = sudo.post("/course", course("CS101", "algo")).await?;
        expect(made.status, 201, "create course")?;
        let algo = made.id()?;
        let twin = sudo.post("/course", course("CS101", "twin")).await?;
        expect(twin.status, 409, "duplicate course code")?;
        let ada = sudo
            .post("/student", json!({ "no": "S01", "name": "ada" }))
            .await?;
        expect(ada.status, 201, "create student")?;
        let ada = ada.id()?;
        let path = format!("/student/{ada}/courses");
        let tie = sudo
            .post(&path, json!({ "right": algo, "grade": "A" }))
            .await?;
        expect(tie.status, 201, "tie student to course")?;
        let tie = tie.id()?;
        grade(&sudo, "A").await?;
        let count = sudo
            .query(&format!(r#"from Student where courses has "{algo}" count"#))
            .await?;
        expect(count.status, 200, "count query")?;
        let count = count.json()?["count"].clone();
        ensure(count == 1, &format!("count is {count}, wanted 1"))?;
        let tuned = sudo
            .patch(&format!("{path}/{tie}"), json!({ "grade": "B" }))
            .await?;
        expect(tuned.status, 204, "tune tie")?;
        grade(&sudo, "B").await?;
        let held = sudo.delete(&format!("/course/{algo}")).await?;
        expect(held.status, 409, "end of a tied course")?;
        self.rename(&sudo, ada).await?;
        let cut = sudo.delete(&format!("{path}/{tie}")).await?;
        expect(cut.status, 204, "cut tie")?;
        let gone = sudo.delete(&format!("/course/{algo}")).await?;
        expect(gone.status, 204, "end of an untied course")?;
        let read = sudo.get(&format!("/course/{algo}")).await?;
        expect(read.status, 404, "read of an ended course")
    }

    async fn rename(&self, sudo: &Client<'_>, ada: i64) -> Result<(), String> {
        let path = format!("/student/{ada}");
        let edit = sudo.patch(&path, json!({ "name": "ada2" })).await?;
        expect(edit.status, 200, "edit student")?;
        ensure(edit.json()?["name"] == "ada2", "edit answered the old name")?;
        let read = sudo.get(&path).await?;
        expect(read.status, 200, "read student")?;
        ensure(read.json()?["name"] == "ada2", "read back the old name")?;
        let list = rows(&sudo.get("/student").await?)?;
        ensure(
            list.len() == 1 && list[0]["name"] == "ada2",
            "list lost the edit",
        )
    }

    pub async fn scoped(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let mut books = Vec::new();
        for tag in ["L01", "L02"] {
            let made = sudo.post("/ledger", json!({ "tag": tag })).await?;
            expect(made.status, 201, "create ledger")?;
            books.push(made.id()?);
        }
        for (note, book) in [("one", books[0]), ("two", books[0]), ("solo", books[1])] {
            let made = sudo
                .post("/entry", json!({ "note": note, "ledger": book }))
                .await?;
            expect(made.status, 201, "create entry")?;
        }
        let held = rows(&sudo.get("/entry").await?)?;
        let mut seen: Vec<(String, Value)> = held
            .iter()
            .map(|row| (row["note"].to_string(), row["index"].clone()))
            .collect();
        seen.sort_by(|a, b| a.0.cmp(&b.0));
        let want = [("\"one\"", 1), ("\"solo\"", 1), ("\"two\"", 2)];
        let same = seen.len() == want.len()
            && seen
                .iter()
                .zip(want)
                .all(|(got, (note, index))| got.0 == note && got.1 == index);
        ensure(same, &format!("entry indexes are {seen:?}"))
    }

    pub async fn spare(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let bare = sudo.post("/note", json!({ "tag": "N01" })).await?;
        expect(bare.status, 201, "create note without body")?;
        let held = sudo
            .post("/note", json!({ "tag": "N02", "body": "held" }))
            .await?;
        expect(held.status, 201, "create note with body")?;
        let bare = sudo.get(&format!("/note/{}", bare.id()?)).await?.json()?;
        ensure(
            bare["body"].is_null(),
            &format!("absent body reads as {}", bare["body"]),
        )?;
        let held = sudo.get(&format!("/note/{}", held.id()?)).await?.json()?;
        ensure(
            held["body"] == "held",
            &format!("held body reads as {}", held["body"]),
        )
    }
}
