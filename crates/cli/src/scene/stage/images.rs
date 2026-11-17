use super::Images;

#[test]
fn immutable() {
    let mut env = vec![
        ("SEATRIAL_IMAGE_ENSIGN".into(), "mutable-template".into()),
        ("SEATRIAL_IMAGE_PLUMB".into(), "other-template".into()),
        ("SEATRIAL_IMAGE".into(), "stale-primary".into()),
        ("KEEP".into(), "retained".into()),
    ];
    Images {
        primary: Some("runtime@sha256:111".into()),
        peers: vec![
            ("ensign".into(), "service@sha256:222".into()),
            ("plumb".into(), "runtime@sha256:111".into()),
        ],
    }
    .apply(&mut env);
    let count = env.len();
    let env = env
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(count, env.len());
    assert_eq!(env["SEATRIAL_IMAGE"], env["SEATRIAL_IMAGE_PLUMB"]);
    assert_eq!(env["SEATRIAL_IMAGE_ENSIGN"], "service@sha256:222");
    assert_eq!(env["KEEP"], "retained");
}
