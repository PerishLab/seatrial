use std::path::PathBuf;

mod call;
mod close;
pub use call::{Output, Request, call, must, words};

use crate::binary::Stop;

pub const OWNER: &str = "seatrial";

pub struct Lane {
    pub ns: String,
    pub dir: PathBuf,
    pub stop: Stop,
}

#[derive(Default)]
pub struct Spec {
    pub name: String,
    pub image: String,
    pub entry: Option<String>,
    pub user: Option<String>,
    pub env: Vec<String>,
    pub mounts: Vec<String>,
    pub args: Vec<String>,
}

impl Lane {
    pub async fn open(root: PathBuf, stop: Stop) -> Result<Self, String> {
        let ns = format!("st{}", crate::file::nonce(4)?);
        let dir = root.join(&ns);
        std::fs::create_dir_all(&dir).map_err(|error| format!("{}: {error}", dir.display()))?;
        let dir = dir
            .canonicalize()
            .map_err(|error| format!("{}: {error}", dir.display()))?;
        let lane = Self { ns, dir, stop };
        let mut args = words(&["network", "create"]);
        args.extend(lane.labels());
        args.push(lane.ns.clone());
        let result = Request {
            args: &args,
            stop: lane.stop.clone(),
        }
        .must()
        .await;
        if let Err(error) = result {
            return match lane.finish(Err(error)).await {
                Err(error) => Err(error),
                Ok(()) => Err("lane creation failed".into()),
            };
        }
        Ok(lane)
    }

    pub fn labels(&self) -> Vec<String> {
        vec![
            "--label".into(),
            format!("perish.owner={OWNER}"),
            "--label".into(),
            format!("perish.lane={}", self.ns),
        ]
    }

    pub fn host(&self, component: &str) -> String {
        format!("{component}.{}.internal", self.ns)
    }

    pub async fn volume(&self, name: &str) -> Result<String, String> {
        let volume = format!("{}-{name}", self.ns);
        let mut args = words(&["volume", "create"]);
        args.extend(self.labels());
        args.push(volume.clone());
        Request {
            args: &args,
            stop: self.stop.clone(),
        }
        .must()
        .await?;
        Ok(volume)
    }

    fn container(&self, spec: &Spec) -> Vec<String> {
        let mut args = vec!["--network".into(), self.ns.clone()];
        args.extend(self.labels());
        if !spec.name.is_empty() {
            args.extend(["--name".into(), spec.name.clone()]);
            args.extend(["--network-alias".into(), spec.name.clone()]);
        }
        if let Some(entry) = &spec.entry {
            args.extend(["--entrypoint".into(), entry.clone()]);
        }
        if let Some(user) = &spec.user {
            args.extend(["--user".into(), user.clone()]);
        }
        for pair in &spec.env {
            args.extend(["--env".into(), pair.clone()]);
        }
        for mount in &spec.mounts {
            args.extend(["--volume".into(), mount.clone()]);
        }
        args.push(spec.image.clone());
        args.extend(spec.args.iter().cloned());
        args
    }

    pub async fn start(&self, spec: &Spec) -> Result<(), String> {
        let mut args = words(&["run", "--detach"]);
        args.extend(self.container(spec));
        Request {
            args: &args,
            stop: self.stop.clone(),
        }
        .must()
        .await
        .map(drop)
    }

    pub async fn once(&self, spec: &Spec) -> Result<Output, String> {
        let mut args = words(&["run", "--rm"]);
        args.extend(self.container(spec));
        Request {
            args: &args,
            stop: self.stop.clone(),
        }
        .run()
        .await
    }
}
