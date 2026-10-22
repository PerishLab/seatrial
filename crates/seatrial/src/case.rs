use serde_json::{Value, json};

use crate::site::Site;

pub const NAMES: [&str; 5] = ["again", "health", "sudo", "restart", "unique"];

fn ensure(held: bool, claim: &str) -> Result<(), String> {
    if held { Ok(()) } else { Err(claim.to_string()) }
}

fn expect(status: u16, want: u16, claim: &str) -> Result<(), String> {
    ensure(status == want, &format!("{claim}: {status}, wanted {want}"))
}

fn course(code: &str, title: &str) -> String {
    json!({ "code": code, "title": title }).to_string()
}

impl Site<'_> {
    pub async fn trial(&self, case: &str) -> Result<(), String> {
        match case {
            "again" => self.again().await,
            "health" => self.health().await,
            "sudo" => self.sudo().await,
            "restart" => self.reopen().await,
            "unique" => self.unique().await,
            other => Err(format!("unknown case {other}")),
        }
    }

    async fn sealed(&self) -> Result<String, String> {
        ensure(self.bootstrap("custody").await?, "bootstrap was refused")?;
        self.serve().await?;
        Ok(format!("sudo {}", self.token("custody").await?))
    }

    async fn again(&self) -> Result<(), String> {
        ensure(
            self.bootstrap("custody").await?,
            "first bootstrap was refused",
        )?;
        ensure(
            self.held("custody").await?,
            "first bootstrap wrote no custody file",
        )?;
        ensure(
            !self.bootstrap("second").await?,
            "second bootstrap was accepted",
        )?;
        ensure(
            !self.held("second").await?,
            "second bootstrap wrote a custody file",
        )?;
        let (ok, status) = self.keel(&["status"]).await?;
        ensure(ok && status == "sealed", &format!("status is {status:?}"))
    }

    async fn health(&self) -> Result<(), String> {
        self.sealed().await?;
        expect(self.get("/health", None).await?.status, 200, "health")
    }

    async fn sudo(&self) -> Result<(), String> {
        let auth = self.sealed().await?;
        let made = self
            .post("/course", Some(auth.clone()), course("C1", "sudo"))
            .await?;
        expect(made.status, 201, "sudo create")?;
        let row: Value =
            serde_json::from_str(&made.body).map_err(|error| format!("create: {error}"))?;
        let id = row["id"].to_string();
        let one = self.get(&format!("/course/{id}"), Some(auth)).await?;
        expect(one.status, 200, "sudo read")?;
        ensure(one.body.contains("\"sudo\""), "read back lost the title")?;
        let anon = self.post("/course", None, course("C2", "anon")).await?;
        expect(anon.status, 403, "anonymous create")?;
        let forged = self.get("/course", Some("sudo forged".into())).await?;
        expect(forged.status, 401, "forged sudo")
    }

    async fn reopen(&self) -> Result<(), String> {
        let auth = self.sealed().await?;
        let made = self
            .post("/course", Some(auth.clone()), course("C1", "kept"))
            .await?;
        expect(made.status, 201, "create")?;
        self.restart().await?;
        let list = self.get("/course", Some(auth)).await?;
        expect(list.status, 200, "list after restart")?;
        ensure(list.body.contains("\"kept\""), "row lost across restart")
    }

    async fn unique(&self) -> Result<(), String> {
        let auth = self.sealed().await?;
        let first = self
            .post("/course", Some(auth.clone()), course("U1", "first"))
            .await?;
        expect(first.status, 201, "first create")?;
        let twin = self
            .post("/course", Some(auth), course("U1", "twin"))
            .await?;
        expect(twin.status, 409, "duplicate unique code")
    }
}
