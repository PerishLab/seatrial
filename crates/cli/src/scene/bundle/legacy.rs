use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use super::super::manifest::{self, Scene};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    format: u32,
    cases: Vec<String>,
}

pub(super) fn read(root: &Path, bytes: &str) -> Result<Vec<Scene>, String> {
    let index: Index = toml::from_str(bytes).map_err(|error| error.to_string())?;
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
    Ok(scenes)
}
