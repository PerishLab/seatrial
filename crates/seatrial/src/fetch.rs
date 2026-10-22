use std::io::Read;
use std::os::unix::fs::PermissionsExt;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::probe;
use crate::site::Run;

pub async fn keel(run: &Run) -> Result<(), String> {
    let seal = probe::fetch(&run.lane, &run.pins.keel.seal()).await?;
    let seal: Value = serde_json::from_slice(&seal).map_err(|error| format!("seal: {error}"))?;
    let artifact = &seal["artifacts"]["linux-x64"];
    let (Some(sha), Some(url)) = (artifact["sha256"].as_str(), artifact["url"].as_str()) else {
        return Err("seal names no linux-x64 artifact".into());
    };
    let archive = probe::fetch(&run.lane, url).await?;
    let digest: String = Sha256::digest(&archive)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if digest != sha {
        return Err(format!(
            "archive digest {digest} differs from the seal's {sha}"
        ));
    }
    let binary = unpack(&archive)?;
    write(&run.keel, &binary, 0o755)?;
    let model =
        std::fs::read(&run.source).map_err(|error| format!("{}: {error}", run.source.display()))?;
    write(&run.model, &model, 0o644)
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
