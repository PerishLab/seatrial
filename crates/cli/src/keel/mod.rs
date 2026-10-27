mod case;
mod fetch;
mod pg;
mod probe;
mod site;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::pin;
use seatrial_lib::docker::{self, Lane, must, words};
use seatrial_lib::file::nonce;
use site::{Run, Site, Store};
use tokio::task::JoinSet;

pub async fn run(
    pins: pin::Pins,
    only: &[String],
    stop: seatrial_lib::binary::Stop,
) -> Result<bool, String> {
    let cases = select(only)?;
    let lane = Lane::open(PathBuf::from(".local"), stop).await?;
    println!("lane    {} (perish.owner={})", lane.ns, docker::OWNER);
    let secret = nonce(16)?;
    let dir = lane.dir.clone();
    let mut run = Run {
        keel: dir.join("keel"),
        model: dir.join("model.json"),
        source: pins.keel.model.clone(),
        image: String::new(),
        lane,
        pins,
        secret,
    };
    let prepared = provision(&mut run).await;
    if let Err(error) = prepared {
        run.lane.finish(Err(error)).await?;
        return Ok(false);
    }
    let run = Arc::new(run);
    let result = trials(Arc::clone(&run), cases).await;
    run.lane
        .finish(result.as_ref().map(|_| ()).map_err(String::clone))
        .await?;
    result
}

fn select(only: &[String]) -> Result<Vec<&'static str>, String> {
    if only.is_empty() {
        return Ok(case::suite());
    }
    only.iter()
        .map(|name| case::known(name).ok_or_else(|| format!("unknown case {name}")))
        .collect()
}

async fn trials(run: Arc<Run>, cases: Vec<&'static str>) -> Result<bool, String> {
    let mut set = JoinSet::new();
    for case in cases {
        for &store in case::stores(case) {
            let run = Arc::clone(&run);
            set.spawn(async move { judge(&run, case, store).await });
        }
    }
    let mut passed = true;
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(result) => passed &= result,
            Err(error) => {
                set.shutdown().await;
                return Err(format!("case task: {error}"));
            }
        }
    }
    Ok(passed)
}

async fn provision(run: &mut Run) -> Result<(), String> {
    let images = &run.pins.images;
    for image in [&images.postgres, &images.probe, &images.sqlite] {
        must(&words(&["pull", "--quiet", image])).await?;
    }
    probe::start(&run.lane, &run.pins.images.probe).await?;
    pg::start(&run.lane, &run.pins.images.postgres, &run.secret).await?;
    let release = fetch::keel(run).await?;
    let version = release.version.trim_start_matches('v');
    run.image = format!("{}:{version}", run.pins.keel.image);
    must(&words(&["pull", "--quiet", &run.image])).await?;
    let format = "{{index .RepoDigests 0}}";
    let digest = must(&words(&[
        "image", "inspect", "--format", format, &run.image,
    ]))
    .await?;
    let head = release.commit.get(..12).unwrap_or(&release.commit);
    println!(
        "keel    {} at {head}, {} channel",
        release.version, run.pins.keel.channel
    );
    println!(
        "cli     sha256:{} checked against the seal",
        release.archive
    );
    println!("image   {}", digest.text());
    Ok(())
}

async fn judge(run: &Run, case: &'static str, store: Store) -> bool {
    let start = Instant::now();
    let verdict = match Site::open(run, case, store).await {
        Ok(site) => site.trial(case).await,
        Err(error) => Err(error),
    };
    let took = start.elapsed().as_secs_f32();
    let name = format!("{case}/{}", store.name());
    match &verdict {
        Ok(()) => println!("pass    {name:<16} {took:>5.1}s"),
        Err(error) => println!("fail    {name:<16} {took:>5.1}s  {error}"),
    }
    verdict.is_ok()
}
