use std::path::Path;

use super::select;

#[test]
fn catalog() {
    let suites = select(Path::new("../../scenes"), &[]).unwrap();
    assert_eq!(suites.len(), 1);
    assert_eq!(suites[0].scenes.len(), 28);
    let instances = suites[0]
        .scenes
        .iter()
        .map(|scene| {
            scene
                .manifest
                .command
                .iter()
                .filter(|arg| matches!(arg.as_str(), "lite" | "pg"))
                .count()
        })
        .sum::<usize>();
    assert_eq!(instances, 48);
    assert_eq!(suites[0].setup.products, ["keel"]);
    assert_eq!(suites[0].setup.prepare, ["sh", "-eu", "prepare.sh"]);
}

#[test]
fn exact() {
    let root = Path::new("../../scenes");
    let suites = select(root, &["health".into()]).unwrap();
    assert_eq!(suites[0].scenes.len(), 1);
    assert_eq!(suites[0].scenes[0].name, "health");
    assert!(select(root, &["keel/core/health".into()]).is_err());
    assert!(select(root, &["writer".into()]).is_err());
    let writer = select(Path::new("../../scenes/plumb"), &["writer".into()]).unwrap();
    assert_eq!(writer[0].scenes[0].name, "writer");
    assert!(writer[0].source.ends_with("plumb/writer"));
    assert!(writer[0].control.is_none());
}

#[test]
fn missing() {
    assert!(select(Path::new("../../scenes"), &["missing".into()]).is_err());
}

fn fixture(root: &Path, group: &str) {
    let path = root.join(group);
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("suite.toml"), "default=true\n").unwrap();
    std::fs::write(path.join("case.toml"), "timeout=1\ncommand=['sh']\n").unwrap();
}

#[test]
fn ambiguous() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), "first");
    fixture(root.path(), "second");
    assert!(select(root.path(), &["case".into()]).is_err());
    assert!(select(root.path(), &["first/case".into()]).is_ok());
}

#[test]
fn excluded() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), "group");
    std::fs::write(
        root.path().join("group/broken.toml"),
        "timeout=1\ncommand=['sh']\ndefault=false\nbug='Example/repo#1'\n",
    )
    .unwrap();
    let suites = select(root.path(), &[]).unwrap();
    assert_eq!(suites[0].scenes.len(), 1);
    let suites = select(root.path(), &["broken".into()]).unwrap();
    assert_eq!(
        suites[0].scenes[0].manifest.bug.as_deref(),
        Some("Example/repo#1")
    );
}

#[test]
fn indexed() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("bundle.toml"),
        "format=1\ncases=['case.one']\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("case.one.toml"),
        "timeout=1\ncommand=['sh']\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("model.toml"),
        "this is a business asset, not a case",
    )
    .unwrap();
    let suites = select(root.path(), &[]).unwrap();
    assert_eq!(suites.len(), 1);
    assert_eq!(suites[0].scenes[0].name, "case.one");
    assert!(select(root.path(), &["model".into()]).is_err());
    assert!(select(root.path(), &["case".into()]).is_err());
}

#[test]
fn malformed() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("case.toml"), "timeout=1\ncommand=['sh']\n").unwrap();
    for text in [
        "format=1\ncases=[]",
        "format=2\ncases=['case']",
        "format=1\ncases=['case','case']",
        "format=1\ncases=['../case']",
        "format=1\ncases=['']",
        "format=1\ncases=['missing']",
        "format=1\ncases=['case']\nunknown=true",
    ] {
        std::fs::write(root.path().join("bundle.toml"), text).unwrap();
        assert!(select(root.path(), &[]).is_err(), "{text}");
    }
}

#[test]
fn defaults() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("bundle.toml"),
        "format=1\ncases=['case']\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("case.toml"),
        "timeout=1\ncommand=['sh']\ndefault=false\nbug='Example/repo#1'\n",
    )
    .unwrap();
    assert!(select(root.path(), &[]).is_err());
    assert_eq!(
        select(root.path(), &["case".into()]).unwrap()[0]
            .scenes
            .len(),
        1
    );
}

#[test]
fn writer() {
    let suites = select(Path::new("../../scenes/plumb"), &[]).unwrap();
    assert_eq!(suites.len(), 1);
    assert_eq!(suites[0].scenes[0].name, "writer");
    assert_eq!(suites[0].scenes[0].manifest.products, ["plumb"]);
    let optional = select(
        Path::new("../../scenes/plumb"),
        &["lock-joint".into(), "lock-required".into()],
    )
    .unwrap();
    assert_eq!(optional.len(), 1);
    assert!(optional[0].source.ends_with("lock"));
    assert_eq!(optional[0].scenes.len(), 2);
    for scene in &optional[0].scenes {
        assert!(!scene.manifest.default);
        assert_eq!(scene.manifest.products, ["plumb", "ectropy"]);
        assert_eq!(scene.manifest.image.as_deref(), Some("plumb"));
    }
}

#[cfg(unix)]
#[test]
fn links() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(
        outside.path().join("bundle.toml"),
        "format=1\ncases=['case']\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("bundle.toml"),
        root.path().join("bundle.toml"),
    )
    .unwrap();
    assert!(select(root.path(), &[]).is_err());
    std::fs::remove_file(root.path().join("bundle.toml")).unwrap();
    std::fs::write(
        root.path().join("bundle.toml"),
        "format=1\ncases=['case']\n",
    )
    .unwrap();
    std::fs::write(
        outside.path().join("case.toml"),
        "timeout=1\ncommand=['sh']\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("case.toml"),
        root.path().join("case.toml"),
    )
    .unwrap();
    assert!(select(root.path(), &[]).is_err());
}

#[test]
fn catalogues() {
    let whole = select(Path::new("../../scenes"), &[]).unwrap();
    let keel = select(Path::new("../../scenes/keel"), &[]).unwrap();
    assert_eq!(whole[0].source, keel[0].source);
    assert!(whole[0].setup == keel[0].setup);
    let names = |suite: &super::Suite| {
        suite
            .scenes
            .iter()
            .map(|scene| scene.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&whole[0]), names(&keel[0]));
    assert_eq!(
        whole[0].control.as_ref().unwrap().bytes,
        keel[0].control.as_ref().unwrap().bytes
    );
    let native = select(Path::new("../../scenes/keel/native"), &[]).unwrap();
    assert_eq!(names(&native[0]), ["health-lite", "again-lite"]);
    assert!(!native[0].setup.default);
    let concord = select(Path::new("../../scenes/concord"), &[]).unwrap();
    assert_eq!(names(&concord[0]), ["context", "release"]);
    assert!(concord[0].control.is_none());
}

#[test]
fn optional() {
    for (root, id, old) in [
        ("../../scenes/plumb/lock", "lock-joint", "joint"),
        ("../../scenes/radicula", "radicula-redis", "redis"),
        ("../../scenes/dynet", "dynet-control", "control"),
    ] {
        let root = Path::new(root);
        assert!(select(root, &[]).is_err());
        assert!(select(root, &[old.into()]).is_err());
        let suites = select(root, &[id.into()]).unwrap();
        assert_eq!(suites.len(), 1);
        assert_eq!(suites[0].scenes.len(), 1);
        assert_eq!(suites[0].scenes[0].name, id);
        assert!(!suites[0].scenes[0].manifest.default);
        assert_eq!(
            suites[0].control.as_ref().unwrap().path,
            Path::new("suite.toml")
        );
    }
    let root = Path::new("../../scenes/hardrig");
    assert!(select(root, &[]).is_err());
    let suites = select(root, &["tencent".into()]).unwrap();
    assert!(!suites[0].scenes[0].manifest.default);
    assert_eq!(suites[0].setup.products, ["hardrig", "runseal", "plumb"]);
}
