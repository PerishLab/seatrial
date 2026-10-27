use std::path::{Path, PathBuf};

use seatrial_lib::docker::Lane;
use serde::{Deserialize, Serialize};

use crate::site::Run;

#[derive(Deserialize, Serialize)]
pub struct Images {
    pub postgres: String,
    pub sqlite: String,
}

#[derive(Deserialize, Serialize)]
pub struct Pins {
    pub images: Images,
}

#[derive(Deserialize, Serialize)]
pub struct State {
    pub ns: String,
    pub dir: PathBuf,
    pub keel: PathBuf,
    pub model: PathBuf,
    pub image: String,
    pub secret: String,
    pub pins: Pins,
}

impl State {
    pub fn read(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())
    }

    pub fn write(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        seatrial_lib::file::write(&self.dir.join("state.json"), &bytes, 0o600)
    }

    pub fn run(self) -> Run {
        Run {
            lane: Lane {
                ns: self.ns.clone(),
                dir: self.dir.clone(),
                stop: tokio::sync::watch::channel(false).1,
            },
            state: self,
        }
    }
}
