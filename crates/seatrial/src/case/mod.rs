mod blank;
mod estate;
mod ghost;
mod life;
mod race;
mod refs;
mod rows;
mod tenure;

use serde_json::{Value, json};

use crate::probe::Reply;
use crate::site::{Site, Store};

pub const NAMES: [&str; 19] = [
    "again", "health", "sudo", "restart", "unique", "portable", "scoped", "spare", "lease",
    "cascade", "batch", "pulse", "foreign", "revived", "vetoed", "outlive", "linger", "ghost",
    "blank",
];

pub const RACES: [&str; 8] = [
    "rival", "lapse", "dangle", "orphan", "cling", "adopt", "void", "kin",
];

pub const OPEN: [&str; 2] = ["lapse", "void"];

pub fn known(name: &str) -> Option<&'static str> {
    NAMES.into_iter().chain(RACES).find(|known| *known == name)
}

pub fn suite() -> Vec<&'static str> {
    NAMES
        .into_iter()
        .chain(RACES)
        .filter(|name| !OPEN.contains(name))
        .collect()
}

pub fn stores(name: &str) -> &'static [Store] {
    match RACES.contains(&name) {
        true => &[Store::Pg],
        false => &[Store::Lite, Store::Pg],
    }
}

pub(crate) fn ensure(held: bool, claim: &str) -> Result<(), String> {
    if held { Ok(()) } else { Err(claim.to_string()) }
}

pub(crate) fn expect(status: u16, want: u16, claim: &str) -> Result<(), String> {
    ensure(status == want, &format!("{claim}: {status}, wanted {want}"))
}

pub(crate) fn course(code: &str, title: &str) -> Value {
    json!({ "code": code, "title": title })
}

pub(crate) fn rows(reply: &Reply) -> Result<Vec<Value>, String> {
    match reply.json()? {
        Value::Array(rows) => Ok(rows),
        other => Err(format!("wanted rows, got {other}")),
    }
}

impl Site<'_> {
    pub async fn trial(&self, case: &str) -> Result<(), String> {
        match case {
            "again" => self.again().await,
            "health" => self.health().await,
            "sudo" => self.sudo().await,
            "restart" => self.reopen().await,
            "unique" => self.unique().await,
            "portable" => self.portable().await,
            "scoped" => self.scoped().await,
            "spare" => self.spare().await,
            "lease" => self.lease().await,
            "cascade" => self.cascade().await,
            "batch" => self.batch().await,
            "pulse" => self.pulse().await,
            "foreign" => self.foreign().await,
            "revived" => self.revived().await,
            "vetoed" => self.vetoed().await,
            "outlive" => self.outlive().await,
            "linger" => self.linger().await,
            "ghost" => self.ghost().await,
            "blank" => self.blank().await,
            "rival" => self.rival().await,
            "lapse" => self.lapse().await,
            "dangle" => self.dangle().await,
            "orphan" => self.orphan().await,
            "cling" => self.cling().await,
            "adopt" => self.adopt().await,
            "void" => self.void().await,
            "kin" => self.kin().await,
            other => Err(format!("unknown case {other}")),
        }
    }

    pub(crate) async fn sealed(&self) -> Result<String, String> {
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
