use std::path::{Path, PathBuf};

pub(super) struct Root<'a>(pub &'a Path);

impl Root<'_> {
    pub fn directory(&self, name: &str) -> Result<PathBuf, String> {
        if name == "." {
            return Ok(self.0.into());
        }
        self.resolve(name, true)
    }

    pub fn file(&self, name: &str) -> Result<PathBuf, String> {
        self.resolve(name, false)
    }

    pub fn member(&self, path: &Path) -> Result<PathBuf, String> {
        path.strip_prefix(self.0)
            .map(Path::to_path_buf)
            .map_err(|_| {
                format!(
                    "{} is outside group root {}",
                    path.display(),
                    self.0.display()
                )
            })
    }

    fn resolve(&self, name: &str, directory: bool) -> Result<PathBuf, String> {
        validate(name)?;
        let mut path = self.0.to_path_buf();
        for component in name.split('/') {
            path.push(component);
            let kind = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
            if kind.is_symlink() || (!kind.is_file() && !kind.is_dir()) {
                return Err(format!("unsupported bundle path type: {}", path.display()));
            }
        }
        let kind = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if (directory && !kind.is_dir()) || (!directory && !kind.is_file()) {
            return Err(format!(
                "bundle path has the wrong type: {}",
                path.display()
            ));
        }
        Ok(path)
    }
}

fn validate(name: &str) -> Result<(), String> {
    let bounds = (
        name.is_empty(),
        name.len() > 255,
        name.split('/').count() > 16,
    );
    if bounds != (false, false, false) {
        return Err(format!("invalid bundle path {name}"));
    }
    for component in name.split('/') {
        let spelling = !matches!(component, "" | "." | "..")
            && component
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
        if !spelling {
            return Err(format!("invalid bundle path {name}"));
        }
    }
    Ok(())
}
