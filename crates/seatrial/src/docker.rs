use tokio::process::Command;

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

pub async fn call(args: &[String]) -> Result<Output, String> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .await
        .map_err(|error| format!("docker: {error}"))?;
    Ok(Output {
        ok: output.status.success(),
        out: output.stdout,
        err: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
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
