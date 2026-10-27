use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Pins {
    #[serde(flatten)]
    pub products: std::collections::BTreeMap<String, Channel>,
    pub images: std::collections::BTreeMap<String, String>,
}

#[derive(Deserialize)]
pub struct Channel {
    pub channel: String,
    pub authority: String,
    pub image: Option<String>,
}

pub fn read(path: &Path) -> Result<Pins, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

pub fn valid(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
