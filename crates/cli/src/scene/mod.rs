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

struct Batch {
    group: Arc<stage::Group>,
    suite: Arc<plan::Suite>,
    images: Vec<Option<String>>,
}

pub struct Request<'a> {
    pub pins: &'a Pins,
    pub root: &'a Path,
    pub names: &'a [String],
}

pub async fn run(
    input: Request<'_>,
    stop: Stop,
    record: &mut crate::record::Record,
) -> Result<bool, String> {
    let start = Instant::now();
    let selected = plan::select(input.root, input.names);
    record.stage("selection", start, selected.is_ok());
    let suites = selected?;
    record.selected = suites
        .iter()
        .flat_map(|suite| suite.scenes.iter().map(|scene| scene.name.clone()))
        .collect();
    let start = Instant::now();
    let opened = Lane::open(PathBuf::from(".local"), stop).await;
    record.stage("lane", start, opened.is_ok());
    let lane = Arc::new(opened?);
    record.lane = Some(lane.ns.clone());
    println!("lane    {} (perish.owner=seatrial)", lane.ns);
    let result = execute(input.pins, suites, &lane, record).await;
    let start = Instant::now();
    let cleanup = lane.finish(Ok(())).await;
    record.stage("cleanup", start, cleanup.is_ok());
    record.cleanup = Some(cleanup.is_ok());
    match (result, cleanup) {
        (Err(original), Err(cleanup)) => Err(format!("{original}; cleanup: {cleanup}")),
        (Err(original), Ok(())) => Err(original),
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Ok(passed), Ok(())) => Ok(passed),
    }
}

async fn execute(
    pins: &Pins,
    suites: Vec<plan::Suite>,
    lane: &Arc<Lane>,
    record: &mut crate::record::Record,
) -> Result<bool, String> {
    let batches = prepare(pins, suites, lane, record).await?;
    let start = Instant::now();
    let result = cases(batches, lane, record).await;
    record.stage("execution", start, matches!(&result, Ok(true)));
    result
}

async fn prepare(
    pins: &Pins,
    suites: Vec<plan::Suite>,
    lane: &Arc<Lane>,
    record: &mut crate::record::Record,
) -> Result<Vec<Batch>, String> {
    let mut products = BTreeSet::new();
    for suite in &suites {
        products.extend(suite.setup.products.iter().cloned());
        for scene in &suite.scenes {
            products.extend(scene.manifest.products.iter().cloned());
        }
    }
    let start = Instant::now();
    let prepared = inputs::prepare(pins, &products, lane, record).await;
    record.stage("artifacts", start, prepared.is_ok());
    let env = prepared?;
    let start = Instant::now();
    let result = async {
        let mut batches = Vec::new();
        for (index, suite) in suites.into_iter().enumerate() {
            let group = stage::prepare(&suite, lane, &env, index).await?;
            record.groups.push(crate::record::Group {
                digest: group.digest.clone(),
                cases: suite
                    .scenes
                    .iter()
                    .map(|scene| scene.name.clone())
                    .collect(),
            });
            let images = images(pins, &suite, lane, record).await?;
            batches.push(Batch {
                group: Arc::new(group),
                suite: Arc::new(suite),
                images,
            });
        }
        Ok::<_, String>(batches)
    }
    .await;
    record.stage("preparation", start, result.is_ok());
    result
}

async fn images(
    pins: &Pins,
    suite: &plan::Suite,
    lane: &Lane,
    record: &mut crate::record::Record,
) -> Result<Vec<Option<String>>, String> {
    let mut images = Vec::new();
    for scene in &suite.scenes {
        let image = match &scene.manifest.image {
            Some(name) => Some(inputs::image(pins, name, lane, record).await?),
            None => None,
        };
        images.push(image);
    }
    Ok(images)
}

async fn cases(
    batches: Vec<Batch>,
    lane: &Arc<Lane>,
    record: &mut crate::record::Record,
) -> Result<bool, String> {
    let mut set = JoinSet::new();
    for Batch {
        group,
        suite,
        images,
    } in batches
    {
        for (index, image) in images.into_iter().enumerate() {
            let (group, suite, lane) = (Arc::clone(&group), Arc::clone(&suite), Arc::clone(lane));
            set.spawn(async move { judge(&suite, &group, &lane, (index, image)).await });
        }
    }
    let mut passed = true;
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(result) => {
                passed &= result.passed;
                record.cases.push(result);
            }
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
) -> crate::record::Case {
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
    crate::record::Case {
        name: scene.name.clone(),
        passed: result.is_ok(),
        seconds: start.elapsed().as_secs_f64(),
        error: result.err(),
    }
}
