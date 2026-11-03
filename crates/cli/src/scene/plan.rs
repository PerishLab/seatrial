use std::path::{Path, PathBuf};

use super::manifest::Scene;

mod mapped;
mod setup;

pub use setup::Setup;

#[cfg(test)]
mod strict;
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
    mapped::select(super::bundle::read(&root)?, names)
}
