use std::path::Path;

const INDEX: &str = "format=2\n[[groups]]\nid='root'\nroot='.'\nsetup=[]\nbinding='generation'\nprofile='host/v1'\n[[cases]]\nid='stable'\nmanifest='case.one.toml'\ngroup='root'\n";

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("bundle.toml"), INDEX).unwrap();
    std::fs::write(
        root.path().join("case.one.toml"),
        "timeout=1\ncommand=['must-not-execute']\n",
    )
    .unwrap();
    root
}

async fn rejects(root: &Path, names: &[String]) -> String {
    let pins = crate::pin::Pins {
        scenes: None,
        products: Default::default(),
        images: Default::default(),
    };
    let mut record = crate::record::Record::open(None).unwrap();
    let (_sender, stop) = tokio::sync::watch::channel(false);
    let error = super::super::run(
        super::super::Request {
            pins: &pins,
            root,
            names,
        },
        stop,
        &mut record,
    )
    .await
    .unwrap_err();
    let value = serde_json::to_value(record).unwrap();
    assert!(value["lane"].is_null());
    assert!(value["cleanup"].is_null());
    for field in ["selected", "groups", "products", "images", "cases"] {
        assert_eq!(value[field], serde_json::json!([]));
    }
    assert_eq!(value["stages"].as_array().unwrap().len(), 1);
    assert_eq!(value["stages"][0]["name"], "selection");
    assert_eq!(value["stages"][0]["ok"], false);
    error
}

#[tokio::test]
async fn unindexed() {
    let root = fixture();
    std::fs::remove_file(root.path().join("bundle.toml")).unwrap();
    std::fs::write(root.path().join("suite.toml"), "not valid TOML").unwrap();
    let error = rejects(root.path(), &[]).await;
    assert!(error.contains("requires an explicit format2 bundle.toml"));
}

#[tokio::test]
async fn retired() {
    let root = fixture();
    for text in [
        "format=1\ncases=['case.one']\n",
        "format=3\ncases=[]\n",
        "format='2'\ncases=[]\n",
    ] {
        std::fs::write(root.path().join("bundle.toml"), text).unwrap();
        let error = rejects(root.path(), &[]).await;
        assert!(error.contains("format must be 2"), "{error}");
    }
}

#[tokio::test]
async fn aliases() {
    let root = fixture();
    for name in ["case.one", "root/stable", "missing"] {
        let error = rejects(root.path(), &[name.into()]).await;
        assert!(error.contains("requires an exact case ID"), "{error}");
    }
}

#[tokio::test]
async fn obsolete() {
    let root = fixture();
    std::fs::write(
        root.path().join("bundle.toml"),
        INDEX.replace("setup=[]", "setup=['suite.toml']"),
    )
    .unwrap();
    std::fs::write(
        root.path().join("suite.toml"),
        "default=false\nprepare=['must-not-execute']\n",
    )
    .unwrap();
    assert!(rejects(root.path(), &[]).await.contains("default"));
}

#[test]
fn assets() {
    let root = fixture();
    std::fs::write(root.path().join("suite.toml"), "not valid TOML").unwrap();
    std::fs::write(root.path().join("model.toml"), "not a case").unwrap();
    let suites = super::select(root.path(), &[]).unwrap();
    assert_eq!(suites.len(), 1);
    assert!(suites[0].control.is_none());
    assert!(suites[0].setup.prepare.is_empty());
    assert_eq!(suites[0].scenes[0].name, "stable");
}
