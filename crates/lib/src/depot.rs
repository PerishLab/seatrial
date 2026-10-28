use std::io::Read;
use std::path::{Component, Path, PathBuf};

use plumb::depot::v3::{Generation, Kind, Manifest, Pointer, Query};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub authority: String,
    pub product: String,
    pub channel: String,
}

#[derive(Debug)]
pub struct Material {
    pub root: PathBuf,
    pub pointer: Pointer,
    pub manifest: Manifest,
}

impl Source {
    pub fn fetch(&self, root: &Path) -> Result<Material, String> {
        let authority = self.checked()?;
        let url = format!("{authority}/channels/{}/scenes/latest.json", self.channel);
        let bytes = pointer(&url)?;
        let pointer = Pointer::parse(&bytes)?;
        if pointer.product != self.product
            || pointer.channel != self.channel
            || pointer.kind != Kind::Scene
        {
            return Err("scene channel names another projection".into());
        }
        let held = Generation::latest(Query {
            source: authority,
            product: &self.product,
            channel: &self.channel,
            version: &pointer.version,
            kind: Kind::Scene,
        })?
        .ok_or("scene channel has no version standing")?;
        if held.pointer != pointer {
            return Err("scene channel disagrees with its version standing".into());
        }
        for object in &held.manifest.objects {
            anchored(&object.path)?;
        }
        std::fs::create_dir(root).map_err(|error| error.to_string())?;
        for object in &held.manifest.objects {
            let bytes = held.read(&object.path)?;
            let path = root.join(&object.path);
            std::fs::create_dir_all(path.parent().ok_or("depot object has no parent")?)
                .map_err(|error| error.to_string())?;
            crate::file::write(&path, &bytes, if object.executable { 0o755 } else { 0o644 })?;
        }
        Ok(Material {
            root: root.into(),
            pointer: held.pointer,
            manifest: held.manifest,
        })
    }

    fn checked(&self) -> Result<&str, String> {
        let authority = self.authority.trim_end_matches('/');
        let uri: ureq::http::Uri = authority
            .parse()
            .map_err(|_| "depot authority is not a URI")?;
        let local = uri.scheme_str() == Some("http")
            && matches!(uri.host(), Some("127.0.0.1" | "localhost"))
            && uri.port_u16().is_some_and(|port| port != 0);
        if uri.query().is_some()
            || uri
                .authority()
                .is_some_and(|part| part.as_str().contains('@'))
        {
            return Err("depot authority must not contain query or credentials".into());
        }
        let transport = uri.scheme_str() == Some("https") || local;
        if !transport || uri.host().is_none() {
            return Err("depot authority must be public HTTPS or a loopback HTTP fixture".into());
        }
        for value in [&self.product, &self.channel] {
            if value.is_empty()
                || !value.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte)
                })
            {
                return Err("depot product and channel must be plain names".into());
            }
        }
        Ok(authority)
    }
}

fn anchored(path: &str) -> Result<(), String> {
    let spelling = (
        path.is_empty(),
        path.contains('\\'),
        path.split('/').any(|part| matches!(part, "" | "." | "..")),
    );
    if spelling != (false, false, false) {
        return Err("depot object path is not canonical and anchored".into());
    }
    if !Path::new(path)
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err("depot object path is not canonical and anchored".into());
    }
    Ok(())
}

fn pointer(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url)
        .header("User-Agent", "seatrial")
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .call()
        .map_err(|error| format!("cannot read scene channel: {error}"))?;
    let mut body = response.into_body();
    let mut bytes = Vec::new();
    body.as_reader()
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read scene pointer: {error}"))?;
    if bytes.len() > 65536 {
        return Err("scene pointer exceeds 64 KiB".into());
    }
    Ok(bytes)
}
