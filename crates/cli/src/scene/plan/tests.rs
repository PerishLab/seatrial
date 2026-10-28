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
fn aliases() {
    let cases = ["health".into(), "keel/core/health".into()];
    let suites = select(Path::new("../../scenes"), &cases).unwrap();
    assert_eq!(suites[0].scenes.len(), 1);
    assert_eq!(suites[0].scenes[0].name, "keel/core/health");
    let writer = select(Path::new("../../scenes"), &["writer".into()]).unwrap();
    assert_eq!(writer[0].scenes[0].name, "plumb/follow/writer");
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
    let suites = select(Path::new("../../scenes/plumb/follow"), &[]).unwrap();
    assert_eq!(suites.len(), 1);
    assert_eq!(suites[0].scenes[0].name, "writer");
    assert_eq!(suites[0].scenes[0].manifest.products, ["plumb"]);
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
