use super::placement;
use std::collections::BTreeMap;
use std::io::{Cursor, Read};

const PACKAGE: usize = 64 * 1024 * 1024;
const PAYLOAD: u64 = 64 * 1024 * 1024;
const EXECUTABLE: u64 = 32 * 1024 * 1024;

pub(super) fn unpack(
    bytes: &[u8],
    paths: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    if bytes.len() > PACKAGE {
        return Err("Debian archive exceeds the package budget".into());
    }
    let payload = data(bytes)?;
    let names: BTreeMap<_, _> = paths
        .iter()
        .map(|(name, path)| (path.as_str(), name))
        .collect();
    let mut archive = tar::Archive::new(payload.as_slice());
    let mut files = BTreeMap::new();
    for entry in archive.entries().map_err(fault)? {
        let mut entry = entry.map_err(fault)?;
        let raw = entry.path().map_err(fault)?.to_string_lossy().into_owned();
        let path = raw.strip_prefix("./").unwrap_or(&raw);
        if matches!(path, "" | ".") && entry.header().entry_type().is_dir() {
            continue;
        }
        if !placement::valid(path.trim_end_matches('/')) {
            let sample: String = path.chars().take(96).collect();
            return Err(format!(
                "Debian payload contains a noncanonical path: {sample:?}"
            ));
        }
        if let Some(name) = names.get(path) {
            selected(&entry, files.contains_key(*name))?;
            let mut content = Vec::new();
            entry.read_to_end(&mut content).map_err(fault)?;
            files.insert((*name).clone(), content);
        }
    }
    if files.len() != paths.len() {
        return Err("Debian payload is missing a selected executable".into());
    }
    Ok(files)
}

fn selected<R: Read>(entry: &tar::Entry<'_, R>, duplicate: bool) -> Result<(), String> {
    let header = entry.header();
    if duplicate
        || !header.entry_type().is_file()
        || header.mode().map_err(fault)? & 0o111 == 0
        || entry.size() > EXECUTABLE
    {
        return Err("Debian executable must be one bounded regular executable file".into());
    }
    Ok(())
}

fn data(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut archive = ar::Archive::new(Cursor::new(bytes));
    let mut version = false;
    let mut payload = None;
    let mut control = false;
    let mut count = 0;
    while let Some(entry) = archive.next_entry() {
        count += 1;
        if count > 3 {
            return Err("Debian archive has surplus members".into());
        }
        let mut entry = entry.map_err(fault)?;
        let name = entry.header().identifier().to_vec();
        match name.as_slice() {
            b"debian-binary" if count == 1 && entry.header().size() == 4 => {
                admit(&mut entry)?;
                version = true;
            }
            b"control.tar.xz" | b"control.tar.gz" if count == 2 => control = true,
            b"data.tar.xz" | b"data.tar.gz" if count == 3 => {
                let decoded = decode(&mut entry, name == b"data.tar.xz")?;
                payload = Some(decoded);
            }
            _ => return Err("unsupported or duplicate Debian member".into()),
        }
    }
    if !version || !control || count != 3 {
        return Err("Debian archive needs version 2.0 and one control and data member".into());
    }
    payload.ok_or_else(|| "Debian archive has no data member".into())
}

fn decode(reader: &mut impl Read, xz: bool) -> Result<Vec<u8>, String> {
    let decoder: Box<dyn Read + '_> = if xz {
        Box::new(lzma_rust2::XzReader::new_mem_limit(reader, true, 65_536))
    } else {
        Box::new(flate2::read::GzDecoder::new(reader))
    };
    let mut bytes = Vec::new();
    decoder
        .take(PAYLOAD + 1)
        .read_to_end(&mut bytes)
        .map_err(fault)?;
    if bytes.len() as u64 > PAYLOAD {
        return Err("Debian data exceeds the decoded payload budget".into());
    }
    Ok(bytes)
}

fn admit(reader: &mut impl Read) -> Result<(), String> {
    let mut value = [0; 4];
    reader.read_exact(&mut value).map_err(fault)?;
    if value != *b"2.0\n" {
        return Err("unsupported Debian package version".into());
    }
    Ok(())
}

fn fault(error: std::io::Error) -> String {
    format!("Debian archive: {error}")
}

#[cfg(test)]
mod tests {
    use super::unpack;
    use std::collections::BTreeMap;
    use std::io::Write;

    fn package(entries: &[(&str, tar::EntryType, u32)]) -> Vec<u8> {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut tar = tar::Builder::new(encoder);
        for (name, kind, mode) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(*kind);
            header.set_size(4);
            header.set_mode(*mode);
            header.set_cksum();
            tar.append_data(&mut header, name, b"data".as_slice())
                .unwrap();
        }
        let data = tar.into_inner().unwrap().finish().unwrap();
        envelope(&data, "data.tar.gz")
    }

    fn envelope(data: &[u8], name: &str) -> Vec<u8> {
        let mut builder = ar::Builder::new(Vec::new());
        for (name, bytes) in [
            ("debian-binary", b"2.0\n".as_slice()),
            ("control.tar.gz", b"not executed".as_slice()),
            (name, data),
        ] {
            let header = ar::Header::new(name.as_bytes().to_vec(), bytes.len() as u64);
            builder.append(&header, bytes).unwrap();
        }
        builder.into_inner().unwrap()
    }

    fn paths() -> BTreeMap<String, String> {
        BTreeMap::from([("tool-api".into(), "usr/bin/tool-api".into())])
    }

    #[test]
    fn complete() {
        let archive = package(&[
            (
                "./lib/systemd/system/tool-api@.service",
                tar::EntryType::Regular,
                0o644,
            ),
            ("./usr/bin/tool-api", tar::EntryType::Regular, 0o755),
        ]);
        assert_eq!(unpack(&archive, &paths()).unwrap()["tool-api"], b"data");
    }

    #[test]
    fn missing() {
        let archive = package(&[("usr/bin/other", tar::EntryType::Regular, 0o755)]);
        assert!(unpack(&archive, &paths()).unwrap_err().contains("missing"));
    }

    #[test]
    fn refused() {
        for (kind, mode) in [
            (tar::EntryType::Symlink, 0o755),
            (tar::EntryType::Regular, 0o644),
        ] {
            let archive = package(&[("usr/bin/tool-api", kind, mode)]);
            assert!(
                unpack(&archive, &paths())
                    .unwrap_err()
                    .contains("regular executable")
            );
        }
        let item = ("usr/bin/tool-api", tar::EntryType::Regular, 0o755);
        assert!(
            unpack(&package(&[item, item]), &paths())
                .unwrap_err()
                .contains("regular executable")
        );
    }

    #[test]
    fn unsupported() {
        for name in ["data.tar.zst", "data.tar.xz"] {
            assert!(unpack(&envelope(b"invalid", name), &paths()).is_err());
        }
        assert!(
            unpack(&vec![0; super::PACKAGE + 1], &paths())
                .unwrap_err()
                .contains("package budget")
        );
    }

    #[test]
    fn oversized() {
        let mut header = tar::Header::new_gnu();
        header.set_path("usr/bin/tool-api").unwrap();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_mode(0o755);
        header.set_size(super::EXECUTABLE + 1);
        header.set_cksum();
        let mut archive = tar::Archive::new(header.as_bytes().as_slice());
        let entry = archive.entries().unwrap().next().unwrap().unwrap();
        assert!(super::selected(&entry, false).is_err());
    }

    #[test]
    fn expanded() {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let bytes = vec![0; 1024 * 1024];
        for _ in 0..65 {
            encoder.write_all(&bytes).unwrap();
        }
        let archive = envelope(&encoder.finish().unwrap(), "data.tar.gz");
        assert!(
            unpack(&archive, &paths())
                .unwrap_err()
                .contains("payload budget")
        );
    }
}
