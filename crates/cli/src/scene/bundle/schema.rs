use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Index {
    pub format: u32,
    pub groups: Vec<Group>,
    pub cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Group {
    pub id: String,
    pub root: String,
    pub setup: Vec<String>,
    pub binding: String,
    pub profile: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Case {
    pub id: String,
    pub manifest: String,
    pub group: String,
}

pub(super) fn identifier(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 64
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(format!("invalid bundle identifier {value}"));
    }
    Ok(())
}
