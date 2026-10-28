mod bundle;
mod inputs;
mod manifest;
mod plan;
mod stage;
mod task;

use crate::pin::Pins;
use seatrial_lib::binary::Stop;
use seatrial_lib::docker::Lane;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinSet;

pub async fn run(pins: &Pins, root: &Path, names: &[String], stop: Stop) -> Result<bool, String> {
    let suites = plan::select(root, names)?;
    let lane = Arc::new(Lane::open(PathBuf::from(".local"), stop).await?);
    println!("lane    {} (perish.owner=seatrial)", lane.ns);
    let result = execute(pins, suites, &lane).await;
    lane.finish(result.as_ref().map(|_| ()).map_err(String::clone))
        .await?;
    result
}

async fn execute(pins: &Pins, suites: Vec<plan::Suite>, lane: &Arc<Lane>) -> Result<bool, String> {
    let mut products = BTreeSet::new();
    for suite in &suites {
        products.extend(suite.setup.products.iter().cloned());
        for scene in &suite.scenes {
            products.extend(scene.manifest.products.iter().cloned());
        }
    }
    let env = inputs::prepare(pins, &products, lane).await?;
    let mut batches = Vec::new();
    for (index, suite) in suites.into_iter().enumerate() {
        let group = stage::prepare(&suite, lane, &env, index).await?;
        let mut images = Vec::new();
        for scene in &suite.scenes {
            let image = match &scene.manifest.image {
                Some(name) => Some(inputs::image(pins, name, lane).await?),
                None => None,
            };
            images.push(image);
        }
        batches.push((Arc::new(group), Arc::new(suite), images));
    }
    let mut set = JoinSet::new();
    for (group, suite, images) in batches {
        for (index, image) in images.into_iter().enumerate() {
            let (group, suite, lane) = (Arc::clone(&group), Arc::clone(&suite), Arc::clone(lane));
            set.spawn(async move { judge(&suite, &group, &lane, (index, image)).await });
        }
    }
    let mut passed = true;
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(result) => passed &= result,
            Err(error) => {
                set.shutdown().await;
                return Err(format!("scene task: {error}"));
            }
        }
    }
    Ok(passed)
}

async fn judge(
    suite: &plan::Suite,
    group: &stage::Group,
    lane: &Lane,
    case: (usize, Option<String>),
) -> bool {
    let scene = &suite.scenes[case.0];
    let start = Instant::now();
    let result = async {
        let mut spec = stage::case(group, scene, lane).await?;
        if let Some(image) = case.1 {
            spec.env.push(("SEATRIAL_IMAGE".into(), image));
        }
        task::execute(&spec, lane.stop.clone()).await
    }
    .await;
    let status = match &result {
        Ok(()) => "pass".to_string(),
        Err(error) => format!("fail {error}"),
    };
    println!(
        "scene   {} {status} {:.3}s",
        scene.name,
        start.elapsed().as_secs_f64()
    );
    result.is_ok()
}
