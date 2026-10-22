use std::io::Read;
use std::os::unix::fs::PermissionsExt;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::probe;
use crate::site::Run;

pub struct Release {
    pub version: String,
    pub commit: String,
    pub archive: String,
}

pub async fn keel(run: &Run) -> Result<Release, String> {
    let channel = run.pins.keel.channel.as_str();
    let pointer = parse(
        &probe::fetch(&run.lane, &run.pins.keel.pointer()).await?,
        "pointer",
    )?;
    let canonical = pointer["schema"] == 1 && pointer["channel"] == channel;
    if !canonical || pointer["product"] != "keel" {
        return Err(format!("the {channel} pointer is not canonical"));
    }
    let named = (pointer["releaseVersion"].as_str(), &pointer["seal"]);
    let (Some(version), Some(url), Some(sha)) =
        (named.0, named.1["url"].as_str(), named.1["sha256"].as_str())
    else {
        return Err(format!("the {channel} pointer names no release and seal"));
    };
    let bytes = probe::fetch(&run.lane, url).await?;
    check(&bytes, sha, "seal")?;
    let seal = parse(&bytes, "seal")?;
    if seal["releaseVersion"] != version || seal["channel"] != channel {
        return Err(format!("the seal is not {version} on {channel}"));
    }
    let artifact = &seal["artifacts"]["linux-x64"];
    let (Some(sha), Some(url)) = (artifact["sha256"].as_str(), artifact["url"].as_str()) else {
        return Err("seal names no linux-x64 artifact".into());
    };
    let archive = probe::fetch(&run.lane, url).await?;
    check(&archive, sha, "archive")?;
    write(&run.keel, &unpack(&archive)?, 0o755)?;
    let model =
        std::fs::read(&run.source).map_err(|error| format!("{}: {error}", run.source.display()))?;
    write(&run.model, &model, 0o644)?;
    Ok(Release {
        version: version.into(),
        commit: seal["commit"].as_str().unwrap_or_default().into(),
        archive: sha.into(),
    })
}

fn parse(bytes: &[u8], what: &str) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|error| format!("{what}: {error}"))
}

fn check(bytes: &[u8], want: &str, what: &str) -> Result<(), String> {
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if digest == want {
        return Ok(());
    }
    Err(format!("{what} digest {digest} differs from {want}"))
}

fn unpack(archive: &[u8]) -> Result<Vec<u8>, String> {
    let mut entries = tar::Archive::new(flate2::read::GzDecoder::new(archive));
    let fault = |error: std::io::Error| format!("archive: {error}");
    for entry in entries.entries().map_err(fault)? {
        let mut entry = entry.map_err(fault)?;
        if entry
            .path()
            .map_err(fault)?
            .file_name()
            .is_some_and(|name| name == "keel")
        {
            let mut binary = Vec::new();
            entry.read_to_end(&mut binary).map_err(fault)?;
            return Ok(binary);
        }
    }
    Err("archive holds no keel binary".into())
}

fn write(path: &std::path::Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let fault = |error: std::io::Error| format!("{}: {error}", path.display());
    std::fs::write(path, bytes).map_err(fault)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(fault)
}
