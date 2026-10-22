use std::time::{Duration, Instant};

use crate::docker::{Output, call, must, words};
use crate::lane::{Lane, Spec};
use crate::site::Run;

pub const NAME: &str = "pg";
const WAIT: Duration = Duration::from_secs(90);

pub async fn start(lane: &Lane, image: &str, secret: &str) -> Result<(), String> {
    let volume = lane.volume(NAME).await?;
    lane.start(&Spec {
        name: lane.host(NAME),
        image: image.into(),
        env: vec![format!("POSTGRES_PASSWORD={secret}")],
        mounts: vec![format!("{volume}:/var/lib/postgresql/data")],
        ..Spec::default()
    })
    .await?;
    let host = lane.host(NAME);
    let probe = words(&[
        "exec",
        &host,
        "pg_isready",
        "-U",
        "postgres",
        "-h",
        "127.0.0.1",
    ]);
    let start = Instant::now();
    while start.elapsed() < WAIT {
        if call(&probe).await?.ok {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(format!("{host} never accepted connections"))
}

pub async fn create(run: &Run, case: &str) -> Result<(), String> {
    let host = run.lane.host(NAME);
    let sql = format!("CREATE DATABASE trial_{case}");
    let args = words(&[
        "exec",
        &host,
        "psql",
        "-U",
        "postgres",
        "-h",
        "127.0.0.1",
        "-c",
        &sql,
    ]);
    must(&args).await.map(drop)
}

pub fn url(run: &Run, case: &str) -> String {
    let host = run.lane.host(NAME);
    format!(
        "postgres://postgres:{}@{host}:5432/trial_{case}",
        run.secret
    )
}

pub async fn psql(run: &Run, case: &str, sql: &str) -> Result<Output, String> {
    let host = run.lane.host(NAME);
    let db = format!("trial_{case}");
    let args = words(&[
        "exec",
        &host,
        "psql",
        "-U",
        "postgres",
        "-h",
        "127.0.0.1",
        "-d",
        &db,
        "-tAc",
        sql,
    ]);
    call(&args).await
}
