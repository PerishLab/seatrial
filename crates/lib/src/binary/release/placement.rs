use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub artifact: Medium,
    pub entry: String,
    pub paths: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Medium {
    #[serde(rename = "linux-x64-deb")]
    Deb,
}

impl Placement {
    pub fn medium(&self) -> &'static str {
        match self.artifact {
            Medium::Deb => "linux-x64-deb",
        }
    }

    pub(super) fn check(&self, names: &BTreeSet<String>) -> Result<(), String> {
        if self.paths.keys().cloned().collect::<BTreeSet<_>>() != *names {
            return Err("placement paths must name exactly the requested executables".into());
        }
        let unique: BTreeSet<_> = self.paths.values().collect();
        if unique.len() != self.paths.len() || unique.iter().any(|path| !valid(path)) {
            return Err("placement paths must be distinct canonical relative paths".into());
        }
        Ok(())
    }
}

pub(super) fn valid(path: &str) -> bool {
    if path.is_empty() || path.len() > 256 {
        return false;
    }
    path.bytes()
        .all(|byte| byte.is_ascii_graphic() && byte != b'\\')
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

#[cfg(test)]
mod tests {
    use super::{Medium, Placement};
    use crate::binary::release::{Product, names};
    use std::collections::BTreeMap;

    fn layout(path: &str) -> Placement {
        Placement {
            artifact: Medium::Deb,
            entry: "tool-api".into(),
            paths: BTreeMap::from([("tool-api".into(), path.into())]),
        }
    }

    fn accepts(layout: &Placement) -> bool {
        names(&Product {
            name: "tool",
            channel: "stable",
            pointer: String::new(),
            additional: &[],
            placement: Some(layout),
        })
        .is_ok()
    }

    #[test]
    fn primary() {
        let placement = layout("usr/bin/tool-api");
        assert!(accepts(&placement));
        assert_eq!(placement.medium(), "linux-x64-deb");
        let names = names(&Product {
            name: "tool",
            channel: "stable",
            pointer: String::new(),
            additional: &[],
            placement: None,
        })
        .unwrap();
        assert!(names.contains("tool"));
        assert!(!names.contains("tool-api"));
    }

    #[test]
    fn paths() {
        for path in [
            "/usr/bin/tool-api",
            "../tool-api",
            "usr/./tool-api",
            "usr//tool-api",
            "./usr/bin/tool-api",
            "",
            "usr/bin/tool api",
        ] {
            assert!(!accepts(&layout(path)), "{path}");
        }
        assert!(!accepts(&layout(&"a".repeat(257))));
        let mut placement = layout("usr/bin/tool-api");
        placement
            .paths
            .insert("tool".into(), "usr/bin/tool-api".into());
        assert!(!accepts(&placement));
        placement.paths.clear();
        assert!(!accepts(&placement));
        placement.paths = BTreeMap::from([
            ("tool-api".into(), "usr/bin/tool-api".into()),
            ("helper".into(), "usr/bin/tool-api".into()),
        ]);
        assert!(
            names(&Product {
                name: "tool",
                channel: "stable",
                pointer: String::new(),
                additional: &["helper".into()],
                placement: Some(&placement),
            })
            .is_err()
        );
    }

    #[test]
    fn declaration() {
        let text = r#"{"artifact":"linux-x64-deb","entry":"tool-api","paths":{"tool-api":"usr/bin/tool-api"}}"#;
        assert!(accepts(&serde_json::from_str::<Placement>(text).unwrap()));
        assert!(
            serde_json::from_str::<Placement>(&text.replace("linux-x64-deb", "linux-arm64-deb"))
                .is_err()
        );
        assert!(
            serde_json::from_str::<Placement>(&text.replace("\"entry\"", "\"program\"")).is_err()
        );
    }
}
