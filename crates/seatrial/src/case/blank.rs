use serde_json::{Value, json};

use super::race::told;
use crate::case::{ensure, expect, rows};
use crate::probe::Reply;
use crate::site::Site;

fn badge(kind: &str, label: &str) -> Value {
    json!({ "kind": kind, "label": label })
}

fn loose(model: &mut Value) -> Result<(), String> {
    let kind = model["units"]
        .as_array_mut()
        .and_then(|units| units.iter_mut().find(|unit| unit["name"] == "Badge"))
        .and_then(|unit| unit["fields"].as_array_mut())
        .and_then(|fields| fields.iter_mut().find(|field| field["name"] == "kind"))
        .ok_or("the model has no Badge.kind")?;
    kind["need"] = json!(false);
    Ok(())
}

fn refused(reply: &Reply) -> bool {
    reply.status == 409
        && reply
            .json()
            .is_ok_and(|body| body == json!({ "error": "field label taken" }))
}

impl Site<'_> {
    pub async fn blank(&self) -> Result<(), String> {
        let mut model = self.model()?;
        loose(&mut model)?;
        let (ok, said) = self.propose(&model).await?;
        println!(
            "blank   a model whose unique scope is optional: {}",
            said.trim()
        );
        ensure(!ok, "a model with an optional unique scope was bound")?;
        ensure(
            said.contains("unique scope kind of label is optional"),
            "the refusal does not name the optional scope",
        )
    }

    pub async fn kin(&self) -> Result<(), String> {
        let (near, far) = self.pair().await?;
        let made = near.post("/badge", badge("k", "B0")).await?;
        expect(made.status, 201, "create badge")?;
        let id = made.id()?;
        let (first, second) = self
            .race(
                near.post("/badge", badge("k", "N1")),
                far.patch(&format!("/badge/{id}"), json!({ "label": "N1" })),
            )
            .await?;
        let list = rows(&near.get("/badge").await?)?;
        let count = list
            .iter()
            .filter(|row| row["label"] == "N1" && row["kind"] == "k")
            .count();
        let told = told(&first, &second);
        ensure(count == 1, &format!("{count} live badges hold N1; {told}"))?;
        let won = [&first, &second]
            .iter()
            .any(|reply| reply.status / 100 == 2);
        let lost = [&first, &second].iter().any(|reply| refused(reply));
        let bodies = format!("{} | {}", first.body, second.body);
        ensure(
            won && lost,
            &format!("wanted one write and one 409 field label taken; {told}: {bodies}"),
        )
    }
}
