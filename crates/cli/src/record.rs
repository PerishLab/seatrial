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
        matches!(
            (
                selected.len() == self.selected.len(),
                selected == reported,
                self.cases.iter().all(|case| case.passed)
            ),
            (true, true, true)
        )
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
mod tests {
    use super::*;

    #[test]
    fn completion() {
        let mut record = Record::open(None).unwrap();
        record.settle(&Ok(true), false);
        assert!(!record.ok);
        record.selected.push("writer".into());
        record.cases.push(Case {
            name: "writer".into(),
            passed: true,
            seconds: 0.1,
            error: None,
        });
        record.settle(&Ok(true), false);
        assert!(!record.ok);
        record.cleanup = Some(true);
        record.settle(&Ok(true), false);
        assert!(record.ok);
        record.settle(&Ok(true), true);
        assert!(!record.ok);
        assert_eq!(record.state, "cancelled");
        record.cleanup = Some(false);
        record.settle(&Err("cleanup refused".into()), false);
        assert!(!record.ok);
        record.cleanup = Some(true);
        record.cases[0].name = "other".into();
        record.settle(&Ok(true), false);
        assert!(!record.ok);
    }

    #[test]
    fn duplicates() {
        let mut record = Record::open(None).unwrap();
        record.cleanup = Some(true);
        record.selected = vec!["writer".into(), "writer".into()];
        for _ in 0..2 {
            record.cases.push(Case {
                name: "writer".into(),
                passed: true,
                seconds: 0.1,
                error: None,
            });
        }
        record.settle(&Ok(true), false);
        assert!(!record.ok);
        record.selected[1] = "reader".into();
        record.settle(&Ok(true), false);
        assert!(!record.ok);
        record.cases[1].name = "reader".into();
        record.cases[1].passed = false;
        record.settle(&Ok(false), false);
        assert!(!record.ok);
        assert_eq!(record.state, "failed");
    }

    #[test]
    fn output() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("result.json");
        let mut record = Record::open(Some(&path)).unwrap();
        record.settle(&Err("preparation failed".into()), false);
        record.write().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["cleanup"], serde_json::Value::Null);
        assert!(Record::open(Some(&path)).is_err());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
