use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use super::manifest::{self, Scene};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    format: u32,
    cases: Vec<String>,
}

pub fn read(root: &Path) -> Result<Option<Vec<Scene>>, String> {
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
    let index: Index = toml::from_str(&bytes).map_err(|error| error.to_string())?;
    if index.format != 1 || index.cases.is_empty() {
        return Err("bundle format must be 1 and cases must not be empty".into());
    }
    let mut unique = BTreeSet::new();
    let mut scenes = Vec::new();
    for name in index.cases {
        if name.is_empty() || !unique.insert(name.clone()) {
            return Err("bundle case names must be nonempty and unique".into());
        }
        scenes.push(manifest::load(root, &name)?);
    }
    Ok(Some(scenes))
}
