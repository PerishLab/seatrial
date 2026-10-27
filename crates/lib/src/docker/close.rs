use super::{Lane, must, words};

impl Lane {
    async fn listed(&self, kind: &str) -> Result<Vec<String>, String> {
        let filter = format!("label=perish.lane={}", self.ns);
        let owner = format!("label=perish.owner={}", super::OWNER);
        let mut args = words(&[
            kind, "ls", "--quiet", "--filter", &filter, "--filter", &owner,
        ]);
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
        let mut errors = Vec::new();
        for kind in ["container", "volume", "network"] {
            if let Err(error) = self.remove(kind).await {
                errors.push(format!("cleanup {kind}: {error}"));
            }
        }
        let mut left = Vec::new();
        for kind in ["container", "volume", "network"] {
            match self.listed(kind).await {
                Ok(ids) => left.extend(ids.into_iter().map(|id| format!("{kind} {id}"))),
                Err(error) => errors.push(format!("inspect {kind}: {error}")),
            }
        }
        if errors.is_empty()
            && left.is_empty()
            && let Err(error) = std::fs::remove_dir_all(&self.dir)
        {
            errors.push(format!("cleanup {}: {error}", self.dir.display()));
        }
        match errors.is_empty() {
            true => Ok(left),
            false => Err(format!(
                "{}; remaining: {}",
                errors.join("; "),
                left.join(", ")
            )),
        }
    }

    async fn remove(&self, kind: &str) -> Result<(), String> {
        let ids = self.listed(kind).await?;
        if ids.is_empty() {
            return Ok(());
        }
        let mut args = vec![kind.to_string(), "rm".into()];
        if kind == "container" {
            args.extend(words(&["--force", "--volumes"]));
        }
        args.extend(ids);
        must(&args).await.map(drop)
    }

    pub async fn finish(&self, result: Result<(), String>) -> Result<(), String> {
        let mut errors: Vec<String> = result.err().into_iter().collect();
        match self.ports().await {
            Ok((count, published)) => {
                println!("ports   {count} lane containers, {published} published host ports");
                if published != 0 {
                    errors.push(format!("lane published {published} host ports"));
                }
            }
            Err(error) => errors.push(format!("inspect ports: {error}")),
        }
        match self.close().await {
            Ok(left) if left.is_empty() => println!(
                "lane    {} torn down; nothing labelled with it remains",
                self.ns
            ),
            Ok(left) => errors.push(format!("lane {} left behind: {}", self.ns, left.join(", "))),
            Err(error) => errors.push(error),
        }
        match errors.is_empty() {
            true => Ok(()),
            false => Err(errors.join("; ")),
        }
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
