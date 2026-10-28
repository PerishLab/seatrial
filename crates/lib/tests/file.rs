use seatrial_lib::file::snapshot;

#[test]
fn stable() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("input"), "before").unwrap();
    let target = root.path().join("target");
    let digest = snapshot(&source, &target).unwrap();
    std::fs::write(source.join("input"), "after").unwrap();
    assert_eq!(
        std::fs::read_to_string(target.join("input")).unwrap(),
        "before"
    );
    assert_ne!(
        digest,
        snapshot(&source, &root.path().join("changed")).unwrap()
    );
}

#[test]
fn identity() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    std::fs::create_dir(&source).unwrap();
    plumb::testing::executable(&source.join("first"), "same bytes");
    let first = snapshot(&source, &root.path().join("first")).unwrap();
    std::fs::rename(source.join("first"), source.join("second")).unwrap();
    let second = snapshot(&source, &root.path().join("second")).unwrap();
    std::fs::set_permissions(
        source.join("second"),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    let third = snapshot(&source, &root.path().join("third")).unwrap();
    assert_ne!(first, second);
    assert_ne!(second, third);
}

#[test]
fn link() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    std::fs::create_dir(&source).unwrap();
    std::os::unix::fs::symlink("/outside", source.join("link")).unwrap();
    assert!(
        snapshot(&source, &root.path().join("target"))
            .unwrap_err()
            .contains("unsupported")
    );
}

#[test]
fn bytes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("binary");
    std::fs::write(&path, b"abc").unwrap();
    assert_eq!(
        seatrial_lib::file::digest(&path).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let link = root.path().join("alias");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(
        seatrial_lib::file::digest(&link)
            .unwrap_err()
            .contains("unsupported")
    );
    let socket = root.path().join("socket");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    assert!(
        seatrial_lib::file::digest(&socket)
            .unwrap_err()
            .contains("unsupported")
    );
}
