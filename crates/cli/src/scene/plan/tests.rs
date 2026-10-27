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
