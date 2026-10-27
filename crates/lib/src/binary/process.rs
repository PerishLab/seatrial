use std::path::PathBuf;
use std::process::{Output, Stdio};
use std::time::Duration;

use tokio::process::Command;
use tokio::sync::watch;

use super::group::Group;

pub type Stop = watch::Receiver<bool>;

pub struct Spec {
    pub command: Vec<String>,
    pub directory: PathBuf,
    pub env: Vec<(String, String)>,
    pub timeout: Duration,
    pub group: bool,
}

pub async fn run(spec: &Spec, mut stop: Stop) -> Result<Output, String> {
    let program = spec.command.first().ok_or("command is empty")?;
    if spec.timeout.is_zero()
        || std::time::Instant::now()
            .checked_add(spec.timeout)
            .is_none()
    {
        return Err("command timeout must be positive and representable".into());
    }
    if *stop.borrow() {
        return Err("command cancelled before launch".into());
    }
    let mut command = Command::new(program);
    command
        .args(&spec.command[1..])
        .current_dir(&spec.directory)
        .envs(spec.env.iter().cloned())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if spec.group {
        command.process_group(0);
    }
    let child = command
        .spawn()
        .map_err(|error| format!("launch {program}: {error}"))?;
    let mut group = Group::new(child.id().ok_or("child has no process id")?, spec.group);
    let wait = child.wait_with_output();
    tokio::pin!(wait);
    let reason = tokio::select! {
        result = &mut wait => {
            group.release();
            return result.map_err(|error| format!("wait {program}: {error}"));
        },
        _ = tokio::time::sleep(spec.timeout) => "command timed out",
        _ = cancelled(&mut stop) => "command cancelled",
    };
    let killed = group.kill();
    let output = tokio::time::timeout(Duration::from_secs(5), wait).await;
    let detail = match output {
        Ok(Ok(output)) => format!(
            "stdout: {}; stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
        Ok(Err(error)) => format!("wait: {error}"),
        Err(_) => "child did not finish after termination".into(),
    };
    match killed {
        Ok(()) => Err(format!("{reason}; {detail}")),
        Err(error) => Err(format!("{reason}; {error}; {detail}")),
    }
}

async fn cancelled(stop: &mut Stop) {
    while !*stop.borrow_and_update() {
        if stop.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}
