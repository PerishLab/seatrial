use std::path::{Path, PathBuf};

use super::manifest::Scene;

mod legacy;
mod mapped;
mod paths;
mod schema;

pub enum Index {
    Legacy(Vec<Scene>),
    Mapped(Vec<Group>),
}

pub struct Group {
    pub id: String,
    pub source: PathBuf,
    pub control: Option<PathBuf>,
    pub scenes: Vec<Scene>,
}

pub fn read(root: &Path) -> Result<Option<Index>, String> {
    let path = root.join("bundle.toml");
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    if !metadata.is_file() {
        return Err("bundle index is not a regular file".into());
    }
    let bytes = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let value: toml::Value = toml::from_str(&bytes).map_err(|error| error.to_string())?;
    let index = match value.get("format").and_then(toml::Value::as_integer) {
        Some(1) => Index::Legacy(legacy::read(root, &bytes)?),
        Some(2) => Index::Mapped(mapped::read(root, &bytes)?),
        _ => return Err("bundle format must be 1 or 2".into()),
    };
    Ok(Some(index))
}

#[cfg(test)]
mod tests;
