use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn nonce(size: usize) -> Result<String, String> {
    let mut bytes = vec![0; size];
    getrandom::fill(&mut bytes).map_err(|error| format!("nonce: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn write(path: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let fault = |error: std::io::Error| format!("{}: {error}", path.display());
    std::fs::write(path, bytes).map_err(fault)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(fault)
}

pub fn snapshot(source: &Path, destination: &Path) -> Result<String, String> {
    copy(source, destination)?;
    digest(destination)
}

pub fn digest(path: &Path) -> Result<String, String> {
    use sha2::Digest;
    let mut digest = sha2::Sha256::new();
    let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.is_file() {
        digest.update(std::fs::read(path).map_err(|error| error.to_string())?);
    } else if metadata.is_dir() {
        hash(path, path, &mut digest)?;
    } else {
        return Err(format!("unsupported file type: {}", path.display()));
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn entries(path: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let entries = std::fs::read_dir(path).map_err(|error| error.to_string())?;
    let mut paths = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    paths.sort();
    Ok(paths)
}

fn copy(source: &Path, destination: &Path) -> Result<(), String> {
    std::fs::create_dir(destination).map_err(|error| error.to_string())?;
    for path in entries(source)? {
        let name = path.file_name().ok_or("file has no name")?;
        let target = destination.join(name);
        let kind = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if kind.is_dir() {
            copy(&path, &target)?;
        } else if kind.is_file() {
            std::fs::copy(&path, target).map_err(|error| error.to_string())?;
        } else {
            return Err(format!("unsupported file type: {}", path.display()));
        }
    }
    let permissions = std::fs::metadata(source)
        .map_err(|error| error.to_string())?
        .permissions();
    std::fs::set_permissions(destination, permissions).map_err(|error| error.to_string())?;
    Ok(())
}

fn hash(root: &Path, path: &Path, digest: &mut sha2::Sha256) -> Result<(), String> {
    use sha2::Digest;
    for path in entries(path)? {
        let name = path.strip_prefix(root).map_err(|error| error.to_string())?;
        let name = name.to_str().ok_or("file name is not UTF-8")?;
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        digest.update(metadata.permissions().mode().to_le_bytes());
        if metadata.is_dir() {
            digest.update(b"directory");
            hash(root, &path, digest)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(format!("unsupported file type: {}", path.display()));
        }
        let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
        digest.update(b"file");
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    Ok(())
}
