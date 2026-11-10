use std::path::Path;

use super::select;

#[test]
fn catalog() {
    let suites = select(Path::new("../../scenes"), &[]).unwrap();
    assert_eq!(suites.len(), 2);
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
    assert!(native[0].setup.prepare.is_empty());
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

#[test]
fn bagua() {
    let root = Path::new("../../scenes");
    let expected = [
        "bagua-workspace",
        "bagua-lifecycle",
        "bagua-flow",
        "bagua-config",
        "bagua-entry",
        "bagua-history",
        "bagua-input",
        "bagua-mouse",
        "bagua-display",
        "bagua-interaction",
        "bagua-exit",
    ];
    let whole = select(root, &[]).unwrap();
    let portable = select(&root.join("bagua"), &[]).unwrap();
    let suite = whole
        .iter()
        .find(|suite| suite.source.ends_with("bagua"))
        .unwrap();
    assert_eq!(portable.len(), 1);
    let names = |suite: &super::Suite| {
        suite
            .scenes
            .iter()
            .map(|scene| scene.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(suite), expected);
    assert_eq!(names(&portable[0]), expected);
    assert!(suite.control.is_none());
    assert!(portable[0].control.is_none());
    for scene in &suite.scenes {
        assert!(scene.manifest.default);
        assert_eq!(scene.manifest.products, ["bagua"]);
        assert_eq!(scene.manifest.image.as_deref(), Some("bagua"));
    }
}

#[test]
fn subjects() {
    use plumb::semantics::subjects::{Role, Subjects};
    let root = Path::new("../../scenes/seatrial");
    let names = vec!["seatrial-placement".into(), "seatrial-service".into()];
    let suites = select(root, &names).unwrap();
    assert_eq!(suites.len(), 1);
    assert_eq!(suites[0].scenes.len(), 2);
    let subjects = Subjects::parse(&std::fs::read(root.join("subjects.json")).unwrap()).unwrap();
    let mut paths = Vec::new();
    for case in &subjects.cases {
        for path in &case.evidence {
            assert!(root.join(path).is_file(), "{path}");
            paths.push(path.clone());
        }
    }
    subjects.bind(&names, &paths).unwrap();
    let diagnostic = subjects
        .cases
        .iter()
        .find(|case| case.id == "seatrial-service")
        .unwrap();
    assert_eq!(diagnostic.role, Role::Diagnostic);
    assert_eq!(diagnostic.subjects, ["seatrial"]);
    assert_eq!(subjects.coverage("seatrial").unwrap().len(), 1);
    assert!(subjects.coverage("locus").unwrap().is_empty());
}
