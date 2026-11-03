use super::read;

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
    let scene = read("stable", root.path().join("group/case.toml")).unwrap();
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
    assert!(read("stable", root.path().join("group/case.toml")).is_err());
}

#[test]
fn empty() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), "products=[]\ntimeout=1\ncommand=[]\n");
    assert!(read("stable", root.path().join("group/case.toml")).is_err());
}

#[test]
fn timeout() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), "products=[]\ntimeout=0\ncommand=['sh']\n");
    assert!(read("stable", root.path().join("group/case.toml")).is_err());
}

#[test]
fn duplicate() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        "products=['plumb','plumb']\ntimeout=1\ncommand=['sh']\n",
    );
    assert!(read("stable", root.path().join("group/case.toml")).is_err());
}
