use crate::binary::{self, Spec, Stop};
use std::time::Duration;

pub struct Output {
    pub ok: bool,
    pub out: Vec<u8>,
    pub err: String,
}

impl Output {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.out).trim().to_string()
    }
}

pub struct Request<'a> {
    pub args: &'a [String],
    pub stop: Stop,
}

impl Request<'_> {
    pub async fn run(&self) -> Result<Output, String> {
        let mut command = vec!["docker".into()];
        command.extend_from_slice(self.args);
        let spec = Spec {
            command,
            directory: ".".into(),
            env: Vec::new(),
            timeout: Duration::from_secs(300),
        };
        let output = binary::run(&spec, self.stop.clone()).await?;
        Ok(Output {
            ok: output.status.success(),
            out: output.stdout,
            err: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }

    pub async fn must(&self) -> Result<Output, String> {
        let output = self.run().await?;
        match output.ok {
            true => Ok(output),
            false => Err(format!("docker: {}", output.err)),
        }
    }
}

pub async fn call(args: &[String]) -> Result<Output, String> {
    let (_, stop) = tokio::sync::watch::channel(false);
    Request { args, stop }.run().await
}

pub async fn must(args: &[String]) -> Result<Output, String> {
    let output = call(args).await?;
    if output.ok {
        return Ok(output);
    }
    let verb = args.first().map_or("", String::as_str);
    Err(format!("docker {verb}: {}", output.err))
}

pub fn words(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}
