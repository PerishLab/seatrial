use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::keel::case::{course, ensure, expect, rows};
use crate::keel::probe::Client;
use crate::keel::site::Site;

fn now() -> Result<i64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|span| span.as_secs() as i64)
        .map_err(|error| format!("clock: {error}"))
}

async fn codes(sudo: &Client<'_>) -> Result<Vec<String>, String> {
    let list = rows(&sudo.get("/course").await?)?;
    Ok(list.iter().map(|row| row["code"].to_string()).collect())
}

impl Site<'_> {
    pub async fn lease(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let made = sudo.post("/course", course("L1", "leased")).await?;
        expect(made.status, 201, "create course")?;
        let id = made.id()?;
        let past = json!([{ "verb": "end", "unit": "Course", "id": id, "at": now()? - 60 }]);
        expect(sudo.batch(past).await?.status, 400, "lease in the past")?;
        let until = now()? + 3;
        let lease = json!([{ "verb": "end", "unit": "Course", "id": id, "at": until }]);
        expect(sudo.batch(lease).await?.status, 200, "lease")?;
        let held = sudo.get(&format!("/course/{id}")).await?;
        expect(held.status, 200, "read before the lease ends")?;
        let trace = sudo.get(&format!("/-/keel/row/course/{id}")).await?;
        expect(trace.status, 200, "operator read of the lease")?;
        let expires = trace.json()?["expires_at"].clone();
        ensure(
            expires == until,
            &format!("lease expires at {expires}, wanted {until}"),
        )?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        let gone = sudo.get(&format!("/course/{id}")).await?;
        expect(gone.status, 404, "read after the lease ends")?;
        ensure(
            codes(&sudo).await?.is_empty(),
            "list still holds the leased row",
        )
    }

    pub async fn cascade(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let book = sudo.post("/ledger", json!({ "tag": "C01" })).await?;
        expect(book.status, 201, "create ledger")?;
        let book = book.id()?;
        for note in ["one", "two"] {
            let made = sudo
                .post("/entry", json!({ "note": note, "ledger": book }))
                .await?;
            expect(made.status, 201, "create entry")?;
        }
        let end = sudo.delete(&format!("/ledger/{book}")).await?;
        expect(end.status, 204, "end of a root ledger")?;
        let left = rows(&sudo.get("/entry").await?)?;
        ensure(
            left.is_empty(),
            &format!("{} entries outlived their ledger", left.len()),
        )?;
        let read = sudo.get(&format!("/ledger/{book}")).await?;
        expect(read.status, 404, "read of an ended ledger")
    }

    pub async fn batch(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let twin = json!([
            { "verb": "put", "unit": "Course", "fields": course("B1", "first") },
            { "verb": "put", "unit": "Course", "fields": course("B1", "twin") },
        ]);
        expect(
            sudo.batch(twin).await?.status,
            409,
            "batch with a duplicate",
        )?;
        ensure(
            codes(&sudo).await?.is_empty(),
            "failed batch committed a row",
        )?;
        let pair = json!([
            { "verb": "put", "unit": "Course", "fields": course("B2", "kept") },
            { "verb": "put", "unit": "Note", "fields": { "tag": "B2" } },
        ]);
        let done = sudo.batch(pair).await?;
        expect(done.status, 200, "batch of two puts")?;
        let ids = done.json()?["ids"].clone();
        let ids: Vec<i64> = ids.as_array().map_or(Vec::new(), |list| {
            list.iter().filter_map(Value::as_i64).collect()
        });
        ensure(ids.len() == 2, &format!("batch answered ids {ids:?}"))?;
        let edit = json!([
            { "verb": "set", "unit": "Course", "id": ids[0], "fields": { "title": "moved" } },
            { "verb": "end", "unit": "Note", "id": ids[1] },
        ]);
        expect(sudo.batch(edit).await?.status, 200, "batch of set and end")?;
        let read = sudo.get(&format!("/course/{}", ids[0])).await?.json()?;
        ensure(read["title"] == "moved", "batch set was not kept")?;
        let note = sudo.get(&format!("/note/{}", ids[1])).await?;
        expect(note.status, 404, "read of a note ended in a batch")
    }

    pub async fn pulse(&self) -> Result<(), String> {
        let sudo = self.admin().await?;
        let made = sudo.post("/course", course("P1", "beat")).await?;
        expect(made.status, 201, "create course")?;
        let id = made.id()?;
        let path = format!("/course/{id}");
        expect(
            sudo.patch(&path, json!({ "title": "beat2" })).await?.status,
            200,
            "edit",
        )?;
        expect(sudo.delete(&path).await?.status, 204, "end")?;
        let flow = sudo.get("/-/keel/pulse?after=0").await?;
        expect(flow.status, 200, "operator read of the pulse")?;
        let beats: Vec<String> = rows(&flow)?
            .iter()
            .filter(|row| row["key"] == id)
            .map(|row| format!("{}:{}", row["verb"], row["unit"]))
            .collect();
        let want = [
            "\"put\":\"course\"",
            "\"set\":\"course\"",
            "\"end\":\"course\"",
        ];
        ensure(
            beats == want,
            &format!("pulse for course {id} is {beats:?}"),
        )
    }
}
