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

#[test]
fn peers() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        "images=['ensign','plumb']\ntimeout=30\ncommand=['sh']\n",
    );
    let scene = read("peers", root.path().join("group/case.toml")).unwrap();
    assert_eq!(scene.manifest.images, ["ensign", "plumb"]);
}

#[test]
fn refused() {
    for images in [
        "['ensign','ensign']",
        "['https://foreign.example/image']",
        "['one','two','three','four','five']",
    ] {
        let root = tempfile::tempdir().unwrap();
        fixture(
            root.path(),
            &format!("images={images}\ntimeout=30\ncommand=['sh']\n"),
        );
        assert!(read("peers", root.path().join("group/case.toml")).is_err());
    }
}
