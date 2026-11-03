use super::*;

#[test]
fn completion() {
    let mut record = Record::open(None).unwrap();
    record.settle(&Ok(true), false);
    assert!(!record.ok);
    record.selected.push("writer".into());
    record.groups.push(Group {
        digest: "a".repeat(64),
        cases: vec!["writer".into()],
    });
    record.cases.push(Case {
        name: "writer".into(),
        passed: true,
        seconds: 0.1,
        error: None,
    });
    record.settle(&Ok(true), false);
    assert!(!record.ok);
    record.cleanup = Some(true);
    record.settle(&Ok(true), false);
    assert!(record.ok);
    record.settle(&Ok(true), true);
    assert!(!record.ok);
    assert_eq!(record.state, "cancelled");
    record.cleanup = Some(false);
    record.settle(&Err("cleanup refused".into()), false);
    assert!(!record.ok);
    record.cleanup = Some(true);
    record.cases[0].name = "other".into();
    record.settle(&Ok(true), false);
    assert!(!record.ok);
}

#[test]
fn duplicates() {
    let mut record = Record::open(None).unwrap();
    record.cleanup = Some(true);
    record.selected = vec!["writer".into(), "writer".into()];
    record.groups.push(Group {
        digest: "a".repeat(64),
        cases: vec!["writer".into(), "reader".into()],
    });
    for _ in 0..2 {
        record.cases.push(Case {
            name: "writer".into(),
            passed: true,
            seconds: 0.1,
            error: None,
        });
    }
    record.settle(&Ok(true), false);
    assert!(!record.ok);
    record.selected[1] = "reader".into();
    record.settle(&Ok(true), false);
    assert!(!record.ok);
    record.cases[1].name = "reader".into();
    record.cases[1].passed = false;
    record.settle(&Ok(false), false);
    assert!(!record.ok);
    assert_eq!(record.state, "failed");
}

#[test]
fn output() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("result.json");
    let mut record = Record::open(Some(&path)).unwrap();
    record.settle(&Err("preparation failed".into()), false);
    record.write().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["cleanup"], serde_json::Value::Null);
    assert!(Record::open(Some(&path)).is_err());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

fn complete() -> Record {
    let mut record = Record::open(None).unwrap();
    record.cleanup = Some(true);
    record.selected = vec!["first".into(), "second".into()];
    record.groups.push(Group {
        digest: "a".repeat(64),
        cases: record.selected.clone(),
    });
    record.cases = record
        .selected
        .iter()
        .map(|name| Case {
            name: name.clone(),
            passed: true,
            seconds: 0.0,
            error: None,
        })
        .collect();
    record
}

#[test]
fn partitions() {
    let mut record = complete();
    record.settle(&Ok(true), false);
    assert!(record.ok);
    let changes: [fn(&mut Record); 6] = [
        |record| record.groups.clear(),
        |record| {
            record.groups[0].cases.pop();
        },
        |record| record.groups[0].cases[0] = "foreign".into(),
        |record| record.groups[0].digest = "g".repeat(64),
        |record| {
            record.groups.push(Group {
                digest: "b".repeat(64),
                cases: vec![],
            })
        },
        |record| {
            record.groups.push(Group {
                digest: "b".repeat(64),
                cases: vec!["first".into()],
            })
        },
    ];
    for change in changes {
        let mut record = complete();
        change(&mut record);
        record.settle(&Ok(true), false);
        assert!(!record.ok);
        assert_eq!(record.state, "error");
    }
    let mut record = complete();
    record.groups[0].cases.pop();
    record.groups.push(Group {
        digest: "a".repeat(64),
        cases: vec!["second".into()],
    });
    record.settle(&Ok(true), false);
    assert!(!record.ok);
}
