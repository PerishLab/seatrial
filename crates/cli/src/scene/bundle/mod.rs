use std::path::{Path, PathBuf};

mod mapped;
mod paths;
mod schema;

pub struct Group {
    pub id: String,
    pub source: PathBuf,
    pub control: Option<PathBuf>,
    pub scenes: Vec<super::manifest::Scene>,
}

pub fn read(root: &Path) -> Result<Vec<Group>, String> {
    let path = root.join("bundle.toml");
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err("Scene root requires an explicit format2 bundle.toml".into());
        }
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    if !metadata.is_file() {
        return Err("bundle index is not a regular file".into());
    }
    let bytes = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let value: toml::Value = toml::from_str(&bytes).map_err(|error| error.to_string())?;
    if value.get("format").and_then(toml::Value::as_integer) != Some(2) {
        return Err("Scene bundle format must be 2; legacy formats are unsupported".into());
    }
    mapped::read(root, &bytes)
}

#[cfg(test)]
mod tests;
