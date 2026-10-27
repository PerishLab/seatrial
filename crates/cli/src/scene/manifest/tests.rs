use super::load;

fn fixture(root: &std::path::Path, text: &str) {
    std::fs::create_dir(root.join("group")).unwrap();
    std::fs::write(root.join("group/case.toml"), text).unwrap();
}

#[test]
fn writer() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        "products=['plumb']\nimage='plumb'\ntimeout=120\ncommand=['sh','-eu','writer.run.sh']\n",
    );
    let scene = load(root.path(), "group/case").unwrap();
    assert_eq!(scene.manifest.products, ["plumb"]);
    assert_eq!(scene.manifest.image.as_deref(), Some("plumb"));
}

#[test]
fn unknown() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        "products=[]\ntimeout=1\ncommand=['sh']\nextra=true\n",
    );
    assert!(load(root.path(), "group/case").is_err());
}

#[test]
fn empty() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), "products=[]\ntimeout=1\ncommand=[]\n");
    assert!(load(root.path(), "group/case").is_err());
}

#[test]
fn timeout() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), "products=[]\ntimeout=0\ncommand=['sh']\n");
    assert!(load(root.path(), "group/case").is_err());
}

#[test]
fn duplicate() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        "products=['plumb','plumb']\ntimeout=1\ncommand=['sh']\n",
    );
    assert!(load(root.path(), "group/case").is_err());
}

#[test]
fn traversal() {
    let root = tempfile::tempdir().unwrap();
    assert!(load(root.path(), "../case").is_err());
}

#[test]
fn escape() {
    let root = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    fixture(source.path(), "products=[]\ntimeout=1\ncommand=['sh']\n");
    std::os::unix::fs::symlink(source.path().join("group"), root.path().join("group")).unwrap();
    assert!(load(root.path(), "group/case").is_err());
}
