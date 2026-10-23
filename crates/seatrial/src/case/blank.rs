use serde_json::{Value, json};

use super::race::told;
use crate::case::{ensure, expect, rows};
use crate::site::Site;

fn badge(kind: Option<&str>, label: &str) -> Value {
    match kind {
        Some(kind) => json!({ "kind": kind, "label": label }),
        None => json!({ "label": label }),
    }
}

fn held(list: &[Value], kind: Option<&str>, label: &str) -> usize {
    let kind = kind.map(Value::from).unwrap_or(Value::Null);
    list.iter()
        .filter(|row| row["label"] == label && row["kind"] == kind)
        .count()
}

impl Site<'_> {
    pub async fn blank(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let first = sudo.post("/badge", badge(None, "B1")).await?;
        expect(first.status, 201, "first badge without a kind")?;
        let twin = sudo.post("/badge", badge(None, "B1")).await?;
        let other = sudo.post("/badge", badge(Some("k"), "B1")).await?;
        println!(
            "blank   a second B1 without a kind answers {}, B1 of kind k answers {}",
            twin.status, other.status
        );
        expect(twin.status, 409, "second badge without a kind")?;
        expect(other.status, 201, "badge of another kind")
    }

    async fn scope(&self, kind: Option<&str>) -> Result<(), String> {
        let (near, far) = self.pair().await?;
        let made = near.post("/badge", badge(kind, "B0")).await?;
        expect(made.status, 201, "create badge")?;
        let id = made.id()?;
        let (first, second) = self
            .race(
                near.post("/badge", badge(kind, "N1")),
                far.patch(&format!("/badge/{id}"), json!({ "label": "N1" })),
            )
            .await?;
        let list = rows(&near.get("/badge").await?)?;
        let count = held(&list, kind, "N1");
        ensure(
            count <= 1,
            &format!("{count} live badges hold N1; {}", told(&first, &second)),
        )
    }

    pub async fn void(&self) -> Result<(), String> {
        self.scope(None).await
    }

    pub async fn kin(&self) -> Result<(), String> {
        self.scope(Some("k")).await
    }
}
