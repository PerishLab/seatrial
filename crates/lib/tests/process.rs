use std::path::Path;
use std::time::Duration;

use seatrial_lib::binary::{Spec, Stop, run};
use tokio::sync::watch;

fn spec(root: &Path, script: &str) -> Spec {
    Spec {
        command: vec!["sh".into(), "-eu".into(), "-c".into(), script.into()],
        directory: root.into(),
        env: vec![("READY".into(), root.join("ready").display().to_string())],
        timeout: Duration::from_secs(30),
    }
}

fn stop() -> Stop {
    watch::channel(false).1
}

async fn ready(path: &Path) -> u32 {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(pid) = std::fs::read_to_string(path)
                .ok()
                .and_then(|text| text.parse().ok())
            {
                return pid;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("child never became ready")
}

async fn dead(pid: u32) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"));
            if stat.is_err()
                || stat.is_ok_and(|stat| {
                    stat.rsplit_once(") ")
                        .is_some_and(|(_, state)| state.starts_with('Z'))
                })
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("owned descendant is still running")
}

#[tokio::test]
async fn output() {
    let root = tempfile::tempdir().unwrap();
    let output = run(
        &spec(root.path(), "printf out; printf err >&2; exit 7"),
        stop(),
    )
    .await
    .unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(output.stdout, b"out");
    assert_eq!(output.stderr, b"err");
}

#[tokio::test]
async fn missing() {
    let root = tempfile::tempdir().unwrap();
    let mut command = spec(root.path(), "exit 0");
    command.command = vec![root.path().join("missing").display().to_string()];
    let error = run(&command, stop()).await.unwrap_err();
    assert!(error.contains("launch"), "{error}");
}

#[tokio::test]
async fn bounds() {
    let root = tempfile::tempdir().unwrap();
    let mut command = spec(root.path(), "exit 0");
    command.timeout = Duration::ZERO;
    assert!(run(&command, stop()).await.unwrap_err().contains("timeout"));
    command.command.clear();
    assert!(run(&command, stop()).await.unwrap_err().contains("empty"));
}

#[tokio::test]
async fn prior() {
    let root = tempfile::tempdir().unwrap();
    let (_, stop) = watch::channel(true);
    let command = spec(root.path(), "printf reached > \"$READY\"");
    assert!(
        run(&command, stop)
            .await
            .unwrap_err()
            .contains("before launch")
    );
    assert!(!root.path().join("ready").exists());
}

#[tokio::test]
async fn timeout() {
    let root = tempfile::tempdir().unwrap();
    let mut command = spec(root.path(), "sleep 30");
    command.timeout = Duration::from_millis(100);
    let error = run(&command, stop()).await.unwrap_err();
    assert!(error.contains("timed out"), "{error}");
}

#[tokio::test]
async fn cancel() {
    let root = tempfile::tempdir().unwrap();
    let command = spec(
        root.path(),
        "sleep 30 & printf '%s' \"$!\" > \"$READY\"; wait",
    );
    let (sender, stop) = watch::channel(false);
    let task = tokio::spawn(async move { run(&command, stop).await });
    let pid = ready(&root.path().join("ready")).await;
    sender.send(true).unwrap();
    let error = task.await.unwrap().unwrap_err();
    assert!(error.contains("cancelled"), "{error}");
    dead(pid).await;
}

#[tokio::test]
async fn drop() {
    let root = tempfile::tempdir().unwrap();
    let command = spec(
        root.path(),
        "sleep 30 & printf '%s' \"$!\" > \"$READY\"; wait",
    );
    let task = tokio::spawn(async move { run(&command, stop()).await });
    let pid = ready(&root.path().join("ready")).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    dead(pid).await;
}

#[tokio::test]
async fn background() {
    let root = tempfile::tempdir().unwrap();
    let command = spec(
        root.path(),
        "sleep 30 >/dev/null 2>&1 & printf '%s' \"$!\" > \"$READY\"",
    );
    assert!(run(&command, stop()).await.unwrap().status.success());
    dead(ready(&root.path().join("ready")).await).await;
}
