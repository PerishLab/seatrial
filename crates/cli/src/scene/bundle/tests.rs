use std::path::Path;

use crate::scene::plan::select;

mod validation;

const INDEX: &str = "format=2\n[[groups]]\nid='root'\nroot='material'\nsetup=[]\nbinding='generation'\nprofile='host/v1'\n[[cases]]\nid='stable'\nmanifest='material/case.one.toml'\ngroup='root'\n";
const CASE: &str = "timeout=1\ncommand=['true']\n";

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("material")).unwrap();
    std::fs::write(root.path().join("material/case.one.toml"), CASE).unwrap();
    std::fs::write(root.path().join("material/suite.toml"), "not a setup file").unwrap();
    std::fs::write(root.path().join("bundle.toml"), INDEX).unwrap();
    root
}

fn index(root: &Path, text: &str) {
    std::fs::write(root.join("bundle.toml"), text).unwrap();
}

#[test]
fn identity() {
    let root = fixture();
    let suites = select(root.path(), &["stable".into()]).unwrap();
    assert_eq!(suites[0].scenes[0].name, "stable");
    assert!(suites[0].scenes[0].path.ends_with("case.one.toml"));
    assert!(suites[0].control.is_none());
    assert!(suites[0].setup.prepare.is_empty());
    for alias in ["case.one", "material/case.one", "root/stable"] {
        assert!(select(root.path(), &[alias.into()]).is_err());
    }
    std::fs::rename(
        root.path().join("material/case.one.toml"),
        root.path().join("material/moved.data"),
    )
    .unwrap();
    index(
        root.path(),
        &INDEX.replace("material/case.one.toml", "material/moved.data"),
    );
    let suites = select(root.path(), &["stable".into()]).unwrap();
    assert_eq!(suites[0].scenes[0].name, "stable");
    assert!(suites[0].scenes[0].path.ends_with("moved.data"));
}

#[test]
fn setup() {
    let root = fixture();
    std::fs::write(
        root.path().join("material/control.toml"),
        "timeout=10\nprepare=['true']\n",
    )
    .unwrap();
    index(
        root.path(),
        &INDEX.replace("setup=[]", "setup=['material/control.toml']"),
    );
    let suites = select(root.path(), &[]).unwrap();
    assert_eq!(suites[0].setup.timeout, 10);
    assert_eq!(suites[0].setup.prepare, ["true"]);
    assert_eq!(
        suites[0].control.as_ref().unwrap().path,
        Path::new("control.toml")
    );
}

#[test]
fn eager() {
    let root = fixture();
    std::fs::write(root.path().join("material/invalid.toml"), "unknown=true").unwrap();
    let cases = "[[cases]]\nid='broken'\nmanifest='material/invalid.toml'\ngroup='root'\n";
    index(root.path(), &format!("{INDEX}{cases}"));
    assert!(select(root.path(), &["stable".into()]).is_err());
    std::fs::create_dir(root.path().join("other")).unwrap();
    std::fs::write(root.path().join("other/control.toml"), "timeout=0").unwrap();
    let group = "[[groups]]\nid='unused'\nroot='other'\nsetup=['other/control.toml']\nbinding='generation'\nprofile='host/v1'\n";
    index(root.path(), &format!("{INDEX}{group}"));
    let error = select(root.path(), &["stable".into()]).err().unwrap();
    assert_eq!(error, "invalid prepare timeout");
}

#[test]
fn defaults() {
    let root = fixture();
    std::fs::write(
        root.path().join("material/case.one.toml"),
        format!("{CASE}default=false\nbug='Example/repo#1'\n"),
    )
    .unwrap();
    assert!(select(root.path(), &[]).is_err());
    assert_eq!(
        select(root.path(), &["stable".into()]).unwrap()[0]
            .scenes
            .len(),
        1
    );
}

#[test]
fn generation() {
    let root = fixture();
    let suites = select(root.path(), &[]).unwrap();
    let group = seatrial_lib::file::digest(&suites[0].source).unwrap();
    let whole = seatrial_lib::file::digest(root.path()).unwrap();
    std::fs::write(root.path().join("unselected.asset"), "changed").unwrap();
    assert_eq!(
        seatrial_lib::file::digest(&suites[0].source).unwrap(),
        group
    );
    assert_ne!(seatrial_lib::file::digest(root.path()).unwrap(), whole);
    index(
        root.path(),
        &INDEX.replace("binding='generation'", "binding='selected'"),
    );
    assert!(select(root.path(), &[]).is_err());
}

#[test]
fn report() {
    let root = fixture();
    let suites = select(root.path(), &[]).unwrap();
    let name = &suites[0].scenes[0].name;
    let digest = seatrial_lib::file::digest(&suites[0].source).unwrap();
    let mut report = crate::record::Record::open(None).unwrap();
    report.selected = vec![name.clone()];
    report.groups.push(crate::record::Group {
        digest: digest.clone(),
        cases: vec![name.clone()],
    });
    report.cases.push(crate::record::Case {
        name: name.clone(),
        passed: true,
        seconds: 0.0,
        error: None,
    });
    report.cleanup = Some(true);
    report.settle(&Ok(true), false);
    let value = serde_json::to_value(report).unwrap();
    assert_eq!(value["schema"], "seatrial.run/v1");
    assert_eq!(value["selected"], serde_json::json!(["stable"]));
    assert_eq!(value["cases"][0]["name"], "stable");
    assert_eq!(
        value["groups"][0],
        serde_json::json!({"digest":digest,"cases":["stable"]})
    );
    assert_eq!(value["ok"], true);
}
