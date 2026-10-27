use super::manifest::Scene;
use super::plan::{Setup, Suite};
use seatrial_lib::binary;
use seatrial_lib::docker::Lane;
use seatrial_lib::file;
use std::path::PathBuf;

pub struct Group {
    pub source: PathBuf,
    pub directory: PathBuf,
    pub env: Vec<(String, String)>,
    pub digest: String,
}

pub async fn prepare(
    suite: &Suite,
    lane: &Lane,
    inputs: &[(String, String)],
    index: usize,
) -> Result<Group, String> {
    let work = lane.dir.join(format!("group-{index}"));
    std::fs::create_dir(&work).map_err(|error| error.to_string())?;
    let directory = work.join("scene");
    let digest = file::snapshot(&suite.source, &directory)?;
    let setup = directory.join("suite.toml");
    if setup.is_file() {
        let text = std::fs::read_to_string(setup).map_err(|error| error.to_string())?;
        let copied: Setup = toml::from_str(&text).map_err(|error| error.to_string())?;
        if copied != suite.setup {
            return Err("suite changed during preparation".into());
        }
    }
    let mut env = inputs.to_vec();
    env.extend([
        ("SEATRIAL_LANE".into(), lane.ns.clone()),
        ("SEATRIAL_WORK".into(), work.display().to_string()),
        ("SEATRIAL_SCENE".into(), directory.display().to_string()),
        ("SEATRIAL_SOURCE".into(), suite.source.display().to_string()),
    ]);
    let group = Group {
        source: suite.source.clone(),
        directory,
        env,
        digest,
    };
    println!("suite   {} sha256:{}", suite.source.display(), group.digest);
    if !suite.setup.prepare.is_empty() {
        let spec = binary::Spec {
            command: suite.setup.prepare.clone(),
            directory: group.directory.clone(),
            env: group.env.clone(),
            timeout: std::time::Duration::from_secs(suite.setup.timeout),
            group: true,
        };
        super::task::execute(&spec, lane.stop.clone()).await?;
        if file::digest(&suite.source)? != group.digest {
            return Err("suite source changed while its prepare command ran".into());
        }
    }
    Ok(group)
}

pub async fn case(group: &Group, scene: &Scene, lane: &Lane) -> Result<binary::Spec, String> {
    let relative = scene
        .path
        .strip_prefix(&group.source)
        .map_err(|error| error.to_string())?;
    let bytes = std::fs::read(group.directory.join(relative)).map_err(|error| error.to_string())?;
    if bytes != scene.bytes {
        return Err("scene manifest changed during preparation".into());
    }
    let name = format!("case-{}", file::nonce(8)?);
    let volume = lane.volume(&name).await?;
    println!("volume  {} {volume}", scene.name);
    let mut env = group.env.clone();
    env.push(("SEATRIAL_VOLUME".into(), volume));
    env.push(("SEATRIAL_CASE".into(), scene.name.clone()));
    println!("scene   {} sha256:{}", scene.name, group.digest);
    Ok(binary::Spec {
        command: scene.manifest.command.clone(),
        directory: group.directory.clone(),
        env,
        timeout: std::time::Duration::from_secs(scene.manifest.timeout),
        group: true,
    })
}
