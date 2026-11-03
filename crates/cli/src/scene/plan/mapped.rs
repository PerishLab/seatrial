use std::collections::BTreeSet;

use super::{Suite, setup};
use crate::scene::bundle::Group;

pub(super) fn select(groups: Vec<Group>, names: &[String]) -> Result<Vec<Suite>, String> {
    let mut suites = Vec::new();
    for group in groups {
        let (setup, control) = setup::read(&group.source, group.control.as_deref())?;
        suites.push(Suite {
            source: group.source,
            setup,
            control,
            scenes: group.scenes,
        });
    }
    let selected: BTreeSet<_> = names.iter().collect();
    for name in &selected {
        if !suites
            .iter()
            .flat_map(|suite| &suite.scenes)
            .any(|scene| &scene.name == *name)
        {
            return Err(format!(
                "bundle2 requires an exact case ID; unknown case {name}"
            ));
        }
    }
    for suite in &mut suites {
        suite.scenes.retain(|scene| {
            if names.is_empty() {
                scene.manifest.default
            } else {
                selected.contains(&scene.name)
            }
        });
    }
    suites.retain(|suite| !suite.scenes.is_empty());
    if suites.is_empty() {
        return Err("no scenes selected".into());
    }
    Ok(suites)
}
