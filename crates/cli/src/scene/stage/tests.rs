use super::*;
use crate::scene::plan;
use std::path::Path;
use std::sync::Arc;

fn fixture(root: &Path, setup: &str) {
    std::fs::create_dir(root.join("material")).unwrap();
    std::fs::write(
        root.join("material/case.toml"),
        "timeout=1\ncommand=['true']\n",
    )
    .unwrap();
    std::fs::write(root.join("material/suite.toml"), "invalid setup asset").unwrap();
    std::fs::write(root.join("bundle.toml"), format!("format=2\n[[groups]]\nid='root'\nroot='material'\nsetup={setup}\nbinding='generation'\nprofile='host/v1'\n[[cases]]\nid='stable'\nmanifest='material/case.toml'\ngroup='root'\n")).unwrap();
}

#[test]
fn absent() {
    let root = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    fixture(root.path(), "[]");
    let suites = plan::select(root.path(), &[]).unwrap();
    let (_sender, stop) = tokio::sync::watch::channel(false);
    let lane = Lane {
        ns: "fixture".into(),
        dir: work.path().into(),
        stop,
    };
    let group = snapshot(&suites[0], &lane, &[], 0).unwrap();
    assert_eq!(group.digest, file::digest(&suites[0].source).unwrap());
    assert_eq!(
        std::fs::read_to_string(group.directory.join("suite.toml")).unwrap(),
        "invalid setup asset"
    );
}

#[test]
fn changed() {
    let root = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    fixture(root.path(), "['material/control.toml']");
    let path = root.path().join("material/control.toml");
    std::fs::write(&path, "timeout=1").unwrap();
    let suites = plan::select(root.path(), &[]).unwrap();
    std::fs::write(path, "timeout=2").unwrap();
    let (_sender, stop) = tokio::sync::watch::channel(false);
    let lane = Lane {
        ns: "fixture".into(),
        dir: work.path().into(),
        stop,
    };
    let error = snapshot(&suites[0], &lane, &[], 0).err().unwrap();
    assert_eq!(error, "suite changed during preparation");
}

#[tokio::test]
async fn identical() {
    let root = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let marker = work.path().join("prepared");
    for name in ["first", "second"] {
        let path = root.path().join(name);
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("case.toml"), "timeout=1\ncommand=['true']\n").unwrap();
        std::fs::write(
            path.join("suite.toml"),
            format!("default=true\nprepare=['touch','{}']\n", marker.display()),
        )
        .unwrap();
    }
    let suites = plan::select(root.path(), &[]).unwrap();
    let (_sender, stop) = tokio::sync::watch::channel(false);
    let lane = Arc::new(Lane {
        ns: "fixture".into(),
        dir: work.path().into(),
        stop,
    });
    let pins = toml::from_str("[images]\n").unwrap();
    let mut record = crate::record::Record::open(None).unwrap();
    let error = crate::scene::prepare(&pins, suites, &lane, &mut record)
        .await
        .err()
        .unwrap();
    assert!(error.contains("same snapshot digest"));
    assert!(!marker.exists());
}
