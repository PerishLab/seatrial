use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::manifest::{self, Scene};

#[cfg(test)]
mod tests;

#[derive(Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub products: Vec<String>,
    #[serde(default)]
    pub prepare: Vec<String>,
    #[serde(default = "budget")]
    pub timeout: u64,
}

pub struct Suite {
    pub source: PathBuf,
    pub setup: Setup,
    pub scenes: Vec<Scene>,
}

pub fn select(root: &Path, names: &[String]) -> Result<Vec<Suite>, String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let indexed = super::bundle::read(&root)?;
    let explicit = indexed.is_some();
    let scenes = match indexed {
        Some(scenes) => scenes,
        None => {
            let mut scenes = Vec::new();
            discover(&root, &root, &mut scenes)?;
            scenes
        }
    };
    let mut selected = BTreeSet::new();
    for name in names {
        let matches: Vec<_> = scenes
            .iter()
            .filter(|scene| {
                scene.name == *name
                    || scene
                        .path
                        .file_stem()
                        .is_some_and(|stem| stem == name.as_str())
            })
            .collect();
        if matches.len() != 1 {
            return Err(format!(
                "case {name} resolves to {} scenes; use a qualified name",
                matches.len()
            ));
        }
        selected.insert(matches[0].name.clone());
    }
    let mut suites: Vec<Suite> = Vec::new();
    for scene in scenes {
        let source = source(&root, &scene.path)?;
        let setup = setup(&source)?;
        let take = match names.is_empty() {
            true => (explicit || setup.default) && scene.manifest.default,
            false => selected.contains(&scene.name),
        };
        if !take {
            continue;
        }
        match suites.iter_mut().find(|suite| suite.source == source) {
            Some(suite) => suite.scenes.push(scene),
            None => suites.push(Suite {
                source,
                setup,
                scenes: vec![scene],
            }),
        }
    }
    if suites.is_empty() {
        return Err("no scenes selected".into());
    }
    Ok(suites)
}

fn discover(root: &Path, path: &Path, scenes: &mut Vec<Scene>) -> Result<(), String> {
    let mut entries = std::fs::read_dir(path)
        .map_err(|error| error.to_string())?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort();
    for path in entries {
        let kind = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if kind.is_dir() {
            discover(root, &path, scenes)?;
        } else if !kind.is_file() {
            return Err(format!("unsupported scene file type: {}", path.display()));
        } else if kind.is_file()
            && path.extension().is_some_and(|suffix| suffix == "toml")
            && path
                .file_name()
                .is_some_and(|name| name != "suite.toml" && name != "bundle.toml")
        {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .with_extension("");
            scenes.push(manifest::load(
                root,
                relative.to_str().ok_or("scene path is not UTF-8")?,
            )?);
        }
    }
    Ok(())
}

fn source(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let parent = path.parent().ok_or("scene has no parent")?;
    let mut current = parent;
    while current.starts_with(root) {
        if current.join("suite.toml").is_file() {
            return Ok(current.into());
        }
        let Some(next) = current.parent() else { break };
        current = next;
    }
    Ok(parent.into())
}

fn setup(source: &Path) -> Result<Setup, String> {
    let path = source.join("suite.toml");
    if !path.exists() {
        return Ok(Setup {
            timeout: budget(),
            ..Setup::default()
        });
    }
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let setup: Setup = toml::from_str(&text).map_err(|error| error.to_string())?;
    manifest::products(&setup.products)?;
    let duration = std::time::Duration::from_secs(setup.timeout);
    if duration.is_zero() || std::time::Instant::now().checked_add(duration).is_none() {
        return Err("invalid prepare timeout".into());
    }
    Ok(setup)
}

fn budget() -> u64 {
    300
}
