use seatrial_lib::binary::{self, Product};
use seatrial_lib::docker::{self, Lane, words};
use seatrial_lib::{file, probe};

use super::manifest::Scene;
use crate::pin::Pins;

pub async fn prepare(pins: &Pins, scene: &Scene, lane: &Lane) -> Result<binary::Spec, String> {
    let directory = lane.dir.join("scene");
    let source = scene.path.parent().ok_or("scene has no parent")?;
    let digest = file::snapshot(source, &directory)?;
    let name = scene.path.file_name().ok_or("scene has no file name")?;
    let snapshot = std::fs::read(directory.join(name)).map_err(|error| error.to_string())?;
    if snapshot != scene.bytes {
        return Err("scene manifest changed during preparation".into());
    }
    println!("scene   {} sha256:{digest}", scene.name);
    let volume = lane.volume("case").await?;
    let mut env = vec![
        ("SEATRIAL_LANE".into(), lane.ns.clone()),
        ("SEATRIAL_VOLUME".into(), volume),
        ("SEATRIAL_SCENE".into(), directory.display().to_string()),
        ("SEATRIAL_WORK".into(), lane.dir.display().to_string()),
    ];
    if !scene.manifest.products.is_empty() {
        docker::Request {
            args: &words(&["pull", "--quiet", &pins.images.probe]),
            stop: lane.stop.clone(),
        }
        .must()
        .await?;
        probe::start(lane, &pins.images.probe).await?;
    }
    for name in &scene.manifest.products {
        env.push(product(pins, name, lane).await?);
    }
    if let Some(name) = &scene.manifest.image {
        let image = pins
            .products
            .get(name)
            .and_then(|product| product.image.as_ref())
            .ok_or_else(|| format!("scene needs [{name}].image configuration"))?;
        docker::Request {
            args: &words(&["pull", "--quiet", image]),
            stop: lane.stop.clone(),
        }
        .must()
        .await?;
        let args = words(&[
            "image",
            "inspect",
            "--format",
            "{{index .RepoDigests 0}}",
            image,
        ]);
        let digest = docker::Request {
            args: &args,
            stop: lane.stop.clone(),
        }
        .must()
        .await?;
        println!("image   {}", digest.text());
        env.push(("SEATRIAL_IMAGE".into(), image.clone()));
    }
    Ok(binary::Spec {
        command: scene.manifest.command.clone(),
        directory,
        env,
        timeout: std::time::Duration::from_secs(scene.manifest.timeout),
    })
}

async fn product(pins: &Pins, name: &str, lane: &Lane) -> Result<(String, String), String> {
    let channel = pins
        .products
        .get(name)
        .ok_or_else(|| format!("scene needs [{name}] configuration"))?;
    let pointer = format!(
        "{}/v1/channels/{}.json",
        channel.authority.trim_end_matches('/'),
        channel.channel
    );
    let path = lane.dir.join(name);
    let release = binary::fetch(
        lane,
        Product {
            name,
            channel: &channel.channel,
            pointer,
        },
        &path,
    )
    .await?;
    println!(
        "binary  {name} {} at {}, archive sha256:{}",
        release.version, release.commit, release.archive
    );
    Ok((
        format!("SEATRIAL_BINARY_{}", name.to_ascii_uppercase()),
        path.display().to_string(),
    ))
}
