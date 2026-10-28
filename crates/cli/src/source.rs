use std::path::{Path, PathBuf};

use crate::pin::Pins;
use seatrial_lib::depot::Material;

pub struct Root {
    pub path: PathBuf,
    pub depot: Option<Material>,
    _hold: Option<tempfile::TempDir>,
}

pub fn prepare(pins: &Pins, local: Option<&Path>) -> Result<Root, String> {
    if let Some(path) = local {
        return Ok(Root {
            path: path.into(),
            depot: None,
            _hold: None,
        });
    }
    let source = pins
        .scenes
        .as_ref()
        .ok_or("no scenes Depot configured; use --scenes for local development")?;
    let hold = tempfile::tempdir().map_err(|error| error.to_string())?;
    let material = source.fetch(&hold.path().join("material"))?;
    if !material.root.join("bundle.toml").is_file() {
        return Err("Depot scene material carries no bundle.toml".into());
    }
    println!(
        "depot   {} {} {} generation {}",
        material.pointer.product,
        material.pointer.channel,
        material.pointer.version,
        material.pointer.generation
    );
    Ok(Root {
        path: material.root.clone(),
        depot: Some(material),
        _hold: Some(hold),
    })
}
