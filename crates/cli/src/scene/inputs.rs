use crate::pin::Pins;
use seatrial_lib::binary::{self, Product};
use seatrial_lib::docker::{self, Lane, words};
use seatrial_lib::probe;
use std::collections::BTreeSet;

pub async fn prepare(
    pins: &Pins,
    products: &BTreeSet<String>,
    lane: &Lane,
) -> Result<Vec<(String, String)>, String> {
    let mut env = Vec::new();
    for (name, image) in &pins.images {
        env.push((
            format!("SEATRIAL_IMAGE_{}", name.to_ascii_uppercase()),
            image.clone(),
        ));
    }
    if !products.is_empty() {
        let image = pins
            .images
            .get("probe")
            .ok_or("artifact fetching needs images.probe")?;
        pull(lane, image).await?;
        probe::start(lane, image).await?;
    }
    for name in products {
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
                additional: &channel.binaries,
            },
            &path,
        )
        .await?;
        println!(
            "binary  {name} {} at {}, archive sha256:{}",
            release.version, release.commit, release.archive
        );
        env.push((
            format!("SEATRIAL_BINARY_{}", name.to_ascii_uppercase()),
            path.display().to_string(),
        ));
        env.push((
            format!("SEATRIAL_BINARIES_{}", name.to_ascii_uppercase()),
            serde_json::to_string(&release.executables).map_err(|error| error.to_string())?,
        ));
        env.push((
            format!("SEATRIAL_VERSION_{}", name.to_ascii_uppercase()),
            release.version,
        ));
        if let Some(image) = &channel.image {
            env.push((
                format!("SEATRIAL_IMAGE_{}", name.to_ascii_uppercase()),
                image.clone(),
            ));
        }
    }
    Ok(env)
}

pub async fn image(pins: &Pins, name: &str, lane: &Lane) -> Result<String, String> {
    let image = pins
        .products
        .get(name)
        .and_then(|product| product.image.as_ref())
        .ok_or_else(|| format!("scene needs [{name}].image configuration"))?;
    pull(lane, image).await?;
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
    Ok(image.clone())
}

async fn pull(lane: &Lane, image: &str) -> Result<(), String> {
    if let Some((_, digest)) = image.split_once("@sha256:") {
        let args = words(&[
            "image",
            "inspect",
            "--format",
            "{{json .RepoDigests}}",
            image,
        ]);
        let observed = docker::Request {
            args: &args,
            stop: lane.stop.clone(),
        }
        .run()
        .await?;
        if observed.ok {
            let digests: Vec<String> =
                serde_json::from_slice(&observed.out).map_err(|error| error.to_string())?;
            if digests
                .iter()
                .any(|value| value.ends_with(&format!("@sha256:{digest}")))
            {
                println!("image   cached {image}");
                return Ok(());
            }
        }
    }
    docker::Request {
        args: &words(&["pull", "--quiet", image]),
        stop: lane.stop.clone(),
    }
    .must()
    .await
    .map(drop)
}
