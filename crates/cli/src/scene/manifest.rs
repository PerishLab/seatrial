use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

#[cfg(test)]
mod tests;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[serde(default)]
    pub products: Vec<String>,
    pub image: Option<String>,
    pub timeout: u64,
    pub command: Vec<String>,
    #[serde(default = "yes")]
    pub default: bool,
    pub bug: Option<String>,
}

pub struct Scene {
    pub name: String,
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub manifest: Manifest,
}

pub fn load(root: &Path, name: &str) -> Result<Scene, String> {
    if name.is_empty()
        || name.contains('\\')
        || !Path::new(name)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(format!("invalid scene name {name}"));
    }
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let path = root.join(format!("{name}.toml"));
    let path = path
        .canonicalize()
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if !path.starts_with(root) {
        return Err("scene escapes its root".into());
    }
    let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
    let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
    let manifest: Manifest = toml::from_str(text).map_err(|error| error.to_string())?;
    validate(&manifest)?;
    Ok(Scene {
        name: name.into(),
        path,
        bytes,
        manifest,
    })
}

fn validate(manifest: &Manifest) -> Result<(), String> {
    if manifest
        .bug
        .as_ref()
        .is_some_and(|bug| bug.trim().is_empty())
    {
        return Err("bug reference is empty".into());
    }
    let timeout = std::time::Duration::from_secs(manifest.timeout);
    if timeout.is_zero() || std::time::Instant::now().checked_add(timeout).is_none() {
        return Err("scene timeout must be positive and representable".into());
    }
    if manifest.command.first().is_none_or(String::is_empty) {
        return Err("scene command is empty".into());
    }
    products(&manifest.products)
}

pub(super) fn products(names: &[String]) -> Result<(), String> {
    let unique: std::collections::BTreeSet<_> = names.iter().collect();
    if unique.len() != names.len() {
        return Err("scene products repeat".into());
    }
    for name in names {
        if !crate::pin::valid(name) {
            return Err(format!("invalid product name {name}"));
        }
    }
    Ok(())
}

fn yes() -> bool {
    true
}
