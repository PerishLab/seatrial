use std::path::PathBuf;

pub(crate) mod docker;

use crate::docker::{Output, call, must, words};

pub const OWNER: &str = "seatrial";

pub struct Lane {
    pub ns: String,
    pub dir: PathBuf,
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
    pub async fn open(root: PathBuf) -> Result<Self, String> {
        let ns = format!("st{}", crate::nonce(4)?);
        let dir = root.join(&ns);
        std::fs::create_dir_all(&dir).map_err(|error| format!("{}: {error}", dir.display()))?;
        let dir = dir
            .canonicalize()
            .map_err(|error| format!("{}: {error}", dir.display()))?;
        let lane = Self { ns, dir };
        let mut args = words(&["network", "create"]);
        args.extend(lane.labels());
        args.push(lane.ns.clone());
        must(&args).await?;
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
        must(&args).await?;
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
        must(&args).await.map(drop)
    }

    pub async fn once(&self, spec: &Spec) -> Result<Output, String> {
        let mut args = words(&["run", "--rm"]);
        args.extend(self.container(spec));
        call(&args).await
    }

    async fn listed(&self, kind: &str) -> Result<Vec<String>, String> {
        let filter = format!("label=perish.lane={}", self.ns);
        let mut args = words(&[kind, "ls", "--quiet", "--filter", &filter]);
        if kind == "container" {
            args.push("--all".into());
        }
        let text = must(&args).await?.text();
        Ok(text.lines().map(str::to_string).collect())
    }

    pub async fn ports(&self) -> Result<(usize, usize), String> {
        let ids = self.listed("container").await?;
        let mut published = 0;
        for id in &ids {
            let args = words(&["inspect", "--format", "{{json .NetworkSettings.Ports}}", id]);
            let text = must(&args).await?.text();
            published += bindings(&text)?;
        }
        Ok((ids.len(), published))
    }

    pub async fn close(&self) -> Result<Vec<String>, String> {
        for kind in ["container", "volume", "network"] {
            let ids = self.listed(kind).await?;
            if ids.is_empty() {
                continue;
            }
            let mut args = vec![kind.to_string(), "rm".into()];
            if kind == "container" {
                args.extend(words(&["--force", "--volumes"]));
            }
            args.extend(ids);
            must(&args).await?;
        }
        let _ = std::fs::remove_dir_all(&self.dir);
        let mut left = Vec::new();
        for kind in ["container", "volume", "network"] {
            let ids = self.listed(kind).await?;
            left.extend(ids.into_iter().map(|id| format!("{kind} {id}")));
        }
        Ok(left)
    }
}

fn bindings(text: &str) -> Result<usize, String> {
    let ports: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("ports: {error}"))?;
    let Some(map) = ports.as_object() else {
        return Ok(0);
    };
    Ok(map
        .values()
        .filter_map(serde_json::Value::as_array)
        .map(Vec::len)
        .sum())
}
