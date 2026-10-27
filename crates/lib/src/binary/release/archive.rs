use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

pub(super) fn unpack(
    archive: &[u8],
    names: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut entries = tar::Archive::new(flate2::read::GzDecoder::new(archive));
    let fault = |error: std::io::Error| format!("archive: {error}");
    let mut files = BTreeMap::new();
    for entry in entries.entries().map_err(fault)? {
        let mut entry = entry.map_err(fault)?;
        let path = entry.path().map_err(fault)?;
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !names.contains(name) {
            continue;
        }
        let name = name.to_owned();
        if !entry.header().entry_type().is_file() || files.contains_key(&name) {
            return Err(format!("archive executable {name} is not one regular file"));
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(fault)?;
        files.insert(name, bytes);
    }
    if files.len() != names.len() {
        return Err("archive is missing a requested executable".into());
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::unpack;
    use std::collections::BTreeSet;

    fn archive(entries: &[(&str, tar::EntryType)]) -> Vec<u8> {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        for (name, kind) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(*kind);
            header.set_size(4);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, name, b"data".as_slice())
                .unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn complete() {
        let names = BTreeSet::from(["tool".to_owned(), "tool-api".to_owned()]);
        let bytes = archive(&[
            ("tool", tar::EntryType::Regular),
            ("bin/tool-api", tar::EntryType::Regular),
        ]);
        let files = unpack(&bytes, &names).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files["tool-api"], b"data");
        assert!(unpack(&archive(&[("tool", tar::EntryType::Regular)]), &names).is_err());
    }

    #[test]
    fn ambiguous() {
        let names = BTreeSet::from(["tool".to_owned()]);
        let bytes = archive(&[
            ("tool", tar::EntryType::Regular),
            ("bin/tool", tar::EntryType::Regular),
        ]);
        assert!(unpack(&bytes, &names).is_err());
        let bytes = archive(&[("tool", tar::EntryType::Symlink)]);
        assert!(unpack(&bytes, &names).is_err());
    }
}
