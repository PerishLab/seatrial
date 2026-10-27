use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod archive;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::docker::Lane;
use crate::file::write;
use crate::probe;

pub struct Release {
    pub version: String,
    pub commit: String,
    pub archive: String,
    pub executables: BTreeMap<String, String>,
}

pub struct Product<'a> {
    pub name: &'a str,
    pub channel: &'a str,
    pub pointer: String,
    pub additional: &'a [String],
}

pub async fn fetch(
    lane: &Lane,
    product: Product<'_>,
    destination: &Path,
) -> Result<Release, String> {
    let names = names(&product)?;
    let channel = product.channel;
    let pointer = parse(&probe::fetch(lane, &product.pointer).await?, "pointer")?;
    let canonical = pointer["schema"] == 1 && pointer["channel"] == channel;
    if !canonical || pointer["product"] != product.name {
        return Err(format!("the {channel} pointer is not canonical"));
    }
    let named = (pointer["releaseVersion"].as_str(), &pointer["seal"]);
    let (Some(version), Some(url), Some(sha)) =
        (named.0, named.1["url"].as_str(), named.1["sha256"].as_str())
    else {
        return Err(format!("the {channel} pointer names no release and seal"));
    };
    let bytes = probe::fetch(lane, url).await?;
    check(&bytes, sha, "seal")?;
    let seal = parse(&bytes, "seal")?;
    if seal["releaseVersion"] != version || seal["channel"] != channel {
        return Err(format!("the seal is not {version} on {channel}"));
    }
    let artifact = &seal["artifacts"]["linux-x64"];
    let (Some(sha), Some(url)) = (artifact["sha256"].as_str(), artifact["url"].as_str()) else {
        return Err("seal names no linux-x64 artifact".into());
    };
    let archive = probe::fetch(lane, url).await?;
    check(&archive, sha, "archive")?;
    let files = archive::unpack(&archive, &names)?;
    let mut executables = BTreeMap::new();
    for (name, bytes) in files {
        let path = if name == product.name {
            destination.to_owned()
        } else {
            destination
                .parent()
                .ok_or("binary destination has no parent")?
                .join(format!("{}-binaries", product.name))
                .join(&name)
        };
        std::fs::create_dir_all(path.parent().ok_or("executable has no parent")?)
            .map_err(|error| error.to_string())?;
        write(&path, &bytes, 0o755)?;
        executables.insert(name, path.display().to_string());
    }
    Ok(Release {
        version: version.into(),
        commit: seal["commit"].as_str().unwrap_or_default().into(),
        archive: sha.into(),
        executables,
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

fn names(product: &Product<'_>) -> Result<BTreeSet<String>, String> {
    if product.additional.len() > 8 {
        return Err("too many additional executables".into());
    }
    let mut names = BTreeSet::new();
    for name in std::iter::once(product.name).chain(product.additional.iter().map(String::as_str)) {
        if !valid(name) || !names.insert(name.to_owned()) {
            return Err(format!("invalid or repeated executable name {name}"));
        }
    }
    Ok(names)
}

fn valid(name: &str) -> bool {
    if name.len() > 64
        || !name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
    {
        return false;
    }
    name.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
    })
}
