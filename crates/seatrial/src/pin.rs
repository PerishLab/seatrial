use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Pins {
    pub keel: Keel,
    pub images: Images,
}

#[derive(Deserialize)]
pub struct Keel {
    pub channel: String,
    pub authority: String,
    pub image: String,
    pub model: PathBuf,
}

#[derive(Deserialize)]
pub struct Images {
    pub postgres: String,
    pub probe: String,
    pub sqlite: String,
}

impl Keel {
    pub fn pointer(&self) -> String {
        let root = self.authority.trim_end_matches('/');
        format!("{root}/v1/channels/{}.json", self.channel)
    }
}

pub fn read(path: &Path) -> Result<Pins, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}
