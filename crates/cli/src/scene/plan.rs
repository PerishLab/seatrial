use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::manifest::{self, Scene};

mod mapped;
mod setup;

pub use setup::Setup;

#[cfg(test)]
mod tests;

pub struct Suite {
    pub source: PathBuf,
    pub setup: Setup,
    pub control: Option<setup::Control>,
    pub scenes: Vec<Scene>,
}

pub fn select(root: &Path, names: &[String]) -> Result<Vec<Suite>, String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let indexed = super::bundle::read(&root)?;
    let explicit = indexed.is_some();
    let scenes = match indexed {
        Some(super::bundle::Index::Mapped(groups)) => return mapped::select(groups, names),
        Some(super::bundle::Index::Legacy(scenes)) => scenes,
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
        let path = source.join("suite.toml");
        let relative = path.exists().then(|| Path::new("suite.toml"));
        let (setup, control) = setup::read(&source, relative)?;
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
                control,
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
