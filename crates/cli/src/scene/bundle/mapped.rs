use std::collections::BTreeSet;
use std::path::Path;

use super::{Group, paths, schema};
use crate::scene::manifest;

pub(super) fn read(root: &Path, bytes: &str) -> Result<Vec<Group>, String> {
    let index: schema::Index = toml::from_str(bytes).map_err(|error| error.to_string())?;
    let sizes = [index.groups.len(), index.cases.len()];
    if index.format != 2 || !sizes.iter().all(|size| (1..=128).contains(size)) {
        return Err("bundle2 requires 1..128 groups and cases".into());
    }
    let mut groups = Vec::new();
    for declared in index.groups {
        let group = group(root, declared)?;
        if groups.iter().any(|other: &Group| {
            other.id == group.id
                || other.source.starts_with(&group.source)
                || group.source.starts_with(&other.source)
        }) {
            return Err(
                "bundle groups need unique IDs and disjoint roots; coalesce identical roots".into(),
            );
        }
        groups.push(group);
    }
    let mut ids = BTreeSet::new();
    let mut files = BTreeSet::new();
    for case in index.cases {
        schema::identifier(&case.id)?;
        if !ids.insert(case.id.clone()) || !files.insert(case.manifest.clone()) {
            return Err("bundle case IDs and manifest paths must be unique".into());
        }
        let group = groups
            .iter_mut()
            .find(|group| group.id == case.group)
            .ok_or_else(|| format!("unknown bundle group {}", case.group))?;
        let path = paths::Root(root).file(&case.manifest)?;
        paths::Root(&group.source).member(&path)?;
        group.scenes.push(manifest::read(&case.id, path)?);
    }
    Ok(groups)
}

fn group(root: &Path, declared: schema::Group) -> Result<Group, String> {
    schema::identifier(&declared.id)?;
    if declared.binding != "generation" || declared.profile != "host/v1" {
        return Err("bundle2 supports only generation binding with host/v1".into());
    }
    if declared.setup.len() > 1 {
        return Err("bundle group setup must be [] or [path]".into());
    }
    let source = paths::Root(root).directory(&declared.root)?;
    let control = declared
        .setup
        .first()
        .map(|name| paths::Root(&source).member(&paths::Root(root).file(name)?))
        .transpose()?;
    Ok(Group {
        id: declared.id,
        source,
        control,
        scenes: Vec::new(),
    })
}
