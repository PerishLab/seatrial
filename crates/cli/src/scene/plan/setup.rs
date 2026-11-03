use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    #[serde(default)]
    pub products: Vec<String>,
    #[serde(default)]
    pub prepare: Vec<String>,
    #[serde(default = "budget")]
    pub timeout: u64,
}

pub struct Control {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

pub(super) fn read(
    source: &Path,
    relative: Option<&Path>,
) -> Result<(Setup, Option<Control>), String> {
    let Some(relative) = relative else {
        return Ok((
            Setup {
                timeout: budget(),
                ..Setup::default()
            },
            None,
        ));
    };
    let bytes = std::fs::read(source.join(relative)).map_err(|error| error.to_string())?;
    let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
    let setup: Setup = toml::from_str(text).map_err(|error| error.to_string())?;
    crate::scene::manifest::products(&setup.products)?;
    let duration = std::time::Duration::from_secs(setup.timeout);
    if duration.is_zero() || std::time::Instant::now().checked_add(duration).is_none() {
        return Err("invalid prepare timeout".into());
    }
    Ok((
        setup,
        Some(Control {
            path: relative.into(),
            bytes,
        }),
    ))
}

fn budget() -> u64 {
    300
}
