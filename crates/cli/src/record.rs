use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::Instant;

use serde::Serialize;

#[derive(Serialize)]
pub struct Artifact {
    pub name: String,
    pub channel: String,
    pub version: String,
    pub commit: String,
    pub seal: String,
    pub archive: String,
}

#[derive(Serialize)]
pub struct Image {
    pub name: String,
    pub reference: String,
    pub digest: String,
}

#[derive(Serialize)]
pub struct Group {
    pub digest: String,
    pub cases: Vec<String>,
}

impl Group {
    fn valid(&self) -> bool {
        let digest = self.digest.len() == 64
            && self
                .digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        digest && !self.cases.is_empty()
    }
}

#[derive(Serialize)]
pub struct Case {
    pub name: String,
    pub passed: bool,
    pub seconds: f64,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct Stage {
    pub name: &'static str,
    pub start: f64,
    pub seconds: f64,
    pub ok: bool,
}

#[derive(Serialize)]
pub struct Record {
    schema: &'static str,
    pub runner: String,
    platform: String,
    pub lane: Option<String>,
    pub scene: Option<plumb::depot::v3::Pointer>,
    pub groups: Vec<Group>,
    pub products: Vec<Artifact>,
    pub images: Vec<Image>,
    pub selected: Vec<String>,
    pub cases: Vec<Case>,
    pub stages: Vec<Stage>,
    pub cleanup: Option<bool>,
    pub ok: bool,
    pub state: &'static str,
    pub error: Option<String>,
    seconds: f64,
    #[serde(skip)]
    started: Instant,
    #[serde(skip)]
    output: Option<File>,
}

impl Record {
    pub fn open(path: Option<&Path>) -> Result<Self, String> {
        let started = Instant::now();
        let output = path
            .map(|path| {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(path)
                    .map_err(|error| format!("report {}: {error}", path.display()))
            })
            .transpose()?;
        let runner = seatrial_lib::file::digest(
            &plumb::config::binary().ok_or("cannot locate runner executable")?,
        )?;
        Ok(Self {
            schema: "seatrial.run/v1",
            runner,
            platform: plumb::config::platform(),
            lane: None,
            scene: None,
            groups: Vec::new(),
            products: Vec::new(),
            images: Vec::new(),
            selected: Vec::new(),
            cases: Vec::new(),
            stages: Vec::new(),
            cleanup: None,
            ok: false,
            state: "error",
            error: None,
            seconds: 0.0,
            started,
            output,
        })
    }

    pub fn stage(&mut self, name: &'static str, start: Instant, ok: bool) {
        self.stages.push(Stage {
            name,
            start: start.duration_since(self.started).as_secs_f64(),
            seconds: start.elapsed().as_secs_f64(),
            ok,
        });
    }

    pub fn settle(&mut self, result: &Result<bool, String>, cancelled: bool) {
        self.seconds = self.started.elapsed().as_secs_f64();
        self.error = result.as_ref().err().cloned();
        let complete = self.coverage() && self.cleanup == Some(true);
        self.ok = matches!((cancelled, result, complete), (false, Ok(true), true));
        self.state = if cancelled {
            "cancelled"
        } else if self.ok {
            "passed"
        } else if matches!(result, Ok(false)) {
            "failed"
        } else {
            "error"
        };
        if !cancelled && matches!(result, Ok(true)) && !self.ok {
            self.error = Some("execution evidence is incomplete".into());
        }
    }

    fn coverage(&self) -> bool {
        if self.selected.is_empty() || self.cases.len() != self.selected.len() {
            return false;
        }
        let selected: std::collections::BTreeSet<_> = self.selected.iter().collect();
        let reported: std::collections::BTreeSet<_> =
            self.cases.iter().map(|case| &case.name).collect();
        if !self.groups(&selected) {
            return false;
        }
        matches!(
            (
                selected.len() == self.selected.len(),
                selected == reported,
                self.cases.iter().all(|case| case.passed)
            ),
            (true, true, true)
        )
    }

    fn groups(&self, selected: &std::collections::BTreeSet<&String>) -> bool {
        let mut digests = std::collections::BTreeSet::new();
        let mut covered = std::collections::BTreeSet::new();
        for group in &self.groups {
            if !group.valid() || !digests.insert(&group.digest) {
                return false;
            }
            for case in &group.cases {
                if !covered.insert(case) {
                    return false;
                }
            }
        }
        &covered == selected
    }

    pub fn write(&mut self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self).map_err(|error| error.to_string())?;
        if let Some(output) = &mut self.output {
            use std::io::Write;
            output
                .write_all(&bytes)
                .and_then(|_| output.sync_all())
                .map_err(|error| format!("report: {error}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
