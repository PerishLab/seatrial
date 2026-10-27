use serde_json::{Value, json};

use crate::keel::probe::{self, Reply};
use seatrial_lib::docker::Lane;
use seatrial_lib::probe::Ask;

pub struct Client<'a> {
    pub lane: &'a Lane,
    pub base: String,
    pub auth: Option<String>,
}

impl Reply {
    pub fn json(&self) -> Result<Value, String> {
        serde_json::from_str(&self.body).map_err(|error| format!("reply {:?}: {error}", self.body))
    }

    pub fn id(&self) -> Result<i64, String> {
        self.json()?["id"]
            .as_i64()
            .ok_or_else(|| format!("reply carries no id: {}", self.body))
    }
}

impl Client<'_> {
    pub async fn call(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Reply, String> {
        let ask = Ask {
            method,
            url: format!("{}{path}", self.base),
            auth: self.auth.clone(),
            body: body.map(|value| value.to_string()),
        };
        probe::send(self.lane, ask).await
    }

    pub async fn get(&self, path: &str) -> Result<Reply, String> {
        self.call("GET", path, None).await
    }

    pub async fn post(&self, path: &str, body: Value) -> Result<Reply, String> {
        self.call("POST", path, Some(body)).await
    }

    pub async fn patch(&self, path: &str, body: Value) -> Result<Reply, String> {
        self.call("PATCH", path, Some(body)).await
    }

    pub async fn delete(&self, path: &str) -> Result<Reply, String> {
        self.call("DELETE", path, None).await
    }

    pub async fn query(&self, text: &str) -> Result<Reply, String> {
        self.post("/query", json!({ "q": text })).await
    }

    pub async fn batch(&self, deeds: Value) -> Result<Reply, String> {
        self.post("/batch", json!({ "deeds": deeds })).await
    }
}
