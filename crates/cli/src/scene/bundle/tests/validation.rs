use super::*;

#[test]
fn fields() {
    let root = fixture();
    for text in [
        INDEX.replace("format=2", "format=3"),
        INDEX.replace("setup=[]\n", ""),
        INDEX.replace("setup=[]", "setup=['one','two']"),
        INDEX.replace("profile='host/v1'", "profile='container-offline/v1'"),
        INDEX.replace("binding='generation'", "binding='selected'"),
        INDEX.replace("group='root'", "group='unknown'"),
        INDEX.replace("profile='host/v1'", "profile='host/v1'\nextra=true"),
        format!("{INDEX}extra=true\n"),
        format!("extra=true\n{INDEX}"),
    ] {
        index(root.path(), &text);
        assert!(select(root.path(), &[]).is_err(), "{text}");
    }
}

#[test]
fn identifiers() {
    let root = fixture();
    for name in [
        "".to_string(),
        "Upper".into(),
        "path/id".into(),
        "case.one".into(),
        "é".into(),
        "x".repeat(65),
    ] {
        index(
            root.path(),
            &INDEX.replace("id='stable'", &format!("id='{name}'")),
        );
        assert!(select(root.path(), &[]).is_err(), "{name}");
    }
    index(
        root.path(),
        &INDEX
            .replace("id='root'", "id='stable'")
            .replace("group='root'", "group='stable'"),
    );
    assert!(select(root.path(), &[]).is_ok());
}

#[test]
fn paths() {
    let root = fixture();
    for path in [
        "/absolute",
        "material//case.one.toml",
        "material/./case.one.toml",
        "material/../case.one.toml",
        "material\\case.one.toml",
        "C:/file",
        "https://host/file",
        "material/",
        ".",
        "material/é",
    ] {
        index(root.path(), &INDEX.replace("material/case.one.toml", path));
        let error = select(root.path(), &[]).err().unwrap();
        assert!(error.contains("invalid bundle path"), "{path}: {error}");
    }
    for path in ["a".repeat(256), vec!["a"; 17].join("/")] {
        index(root.path(), &INDEX.replace("material/case.one.toml", &path));
        assert!(
            select(root.path(), &[])
                .err()
                .unwrap()
                .contains("invalid bundle path")
        );
    }
}

#[test]
fn roots() {
    let root = fixture();
    std::fs::create_dir(root.path().join("material/nested")).unwrap();
    for path in ["material", ".", "material/nested"] {
        let other = format!(
            "[[groups]]\nid='other'\nroot='{path}'\nsetup=[]\nbinding='generation'\nprofile='host/v1'\n"
        );
        index(root.path(), &format!("{INDEX}{other}"));
        assert!(
            select(root.path(), &[])
                .err()
                .unwrap()
                .contains("disjoint roots")
        );
    }
    std::fs::write(root.path().join("outside.toml"), CASE).unwrap();
    index(
        root.path(),
        &INDEX.replace("material/case.one.toml", "outside.toml"),
    );
    assert!(select(root.path(), &[]).is_err());
    std::fs::write(root.path().join("outside.toml"), "timeout=1").unwrap();
    index(
        root.path(),
        &INDEX.replace("setup=[]", "setup=['outside.toml']"),
    );
    assert!(select(root.path(), &[]).is_err());
    index(root.path(), &INDEX.replace("root='material'", "root='.'"));
    assert!(select(root.path(), &[]).is_ok());
}

#[test]
fn duplicates() {
    let root = fixture();
    std::fs::write(root.path().join("material/other.toml"), CASE).unwrap();
    for (id, file) in [("stable", "other"), ("other", "case.one")] {
        let case = format!("[[cases]]\nid='{id}'\nmanifest='material/{file}.toml'\ngroup='root'\n");
        index(root.path(), &format!("{INDEX}{case}"));
        assert!(
            select(root.path(), &[])
                .err()
                .unwrap()
                .contains("must be unique")
        );
    }
}

#[test]
fn limits() {
    let root = fixture();
    let case = "[[cases]]\nid='stable'\nmanifest='material/case.one.toml'\ngroup='root'\n";
    let groups = INDEX.split("[[cases]]").next().unwrap();
    index(root.path(), &format!("{groups}{}", case.repeat(129)));
    assert!(select(root.path(), &[]).err().unwrap().contains("1..128"));
    let group = "[[groups]]\nid='root'\nroot='material'\nsetup=[]\nbinding='generation'\nprofile='host/v1'\n";
    index(
        root.path(),
        &format!("format=2\n{}{case}", group.repeat(129)),
    );
    assert!(select(root.path(), &[]).err().unwrap().contains("1..128"));
}

#[test]
fn links() {
    let root = fixture();
    std::os::unix::fs::symlink(
        root.path().join("material/case.one.toml"),
        root.path().join("material/link.toml"),
    )
    .unwrap();
    index(
        root.path(),
        &INDEX.replace("material/case.one.toml", "material/link.toml"),
    );
    assert!(select(root.path(), &[]).is_err());
    std::os::unix::fs::symlink(root.path().join("material"), root.path().join("linked")).unwrap();
    index(
        root.path(),
        &INDEX.replace("root='material'", "root='linked'"),
    );
    assert!(select(root.path(), &[]).is_err());
    index(
        root.path(),
        &INDEX.replace("material/case.one.toml", "linked/case.one.toml"),
    );
    assert!(select(root.path(), &[]).is_err());
}
