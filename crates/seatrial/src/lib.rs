mod case;
mod docker;
mod fetch;
mod lane;
mod pg;
mod pin;
mod probe;
mod site;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use docker::{must, words};
use lane::Lane;
use site::{Run, Site, Store};
use tokio::task::JoinSet;

const STORES: [Store; 2] = [Store::Lite, Store::Pg];

pub async fn run(pins: &Path, only: &[String]) -> Result<bool, String> {
    let pins = pin::read(pins)?;
    let cases = select(only)?;
    let lane = Lane::open(PathBuf::from(".local")).await?;
    println!("lane    {} (perish.owner={})", lane.ns, lane::OWNER);
    let secret = nonce(16)?;
    let dir = lane.dir.clone();
    let run = Run {
        keel: dir.join("keel"),
        model: dir.join("model.json"),
        source: pins.keel.model.clone(),
        image: String::new(),
        lane,
        pins,
        secret,
    };
    let held = trials(run, cases).await;
    let (run, passed) = held?;
    let (count, published) = run.lane.ports().await?;
    println!("ports   {count} lane containers, {published} published host ports");
    let left = run.lane.close().await?;
    match left.is_empty() {
        true => println!(
            "lane    {} torn down; nothing labelled with it remains",
            run.lane.ns
        ),
        false => println!("lane    {} left behind: {}", run.lane.ns, left.join(", ")),
    }
    Ok(passed && published == 0 && left.is_empty())
}

fn select(only: &[String]) -> Result<Vec<&'static str>, String> {
    if only.is_empty() {
        return Ok(case::NAMES.to_vec());
    }
    only.iter()
        .map(|name| {
            case::NAMES
                .into_iter()
                .find(|known| known == name)
                .ok_or_else(|| format!("unknown case {name}"))
        })
        .collect()
}

async fn trials(mut run: Run, cases: Vec<&'static str>) -> Result<(Run, bool), String> {
    if let Err(error) = provision(&mut run).await {
        let _ = run.lane.close().await;
        return Err(error);
    }
    let run = Arc::new(run);
    let mut set = JoinSet::new();
    for case in cases {
        for store in STORES {
            let run = Arc::clone(&run);
            set.spawn(async move { judge(&run, case, store).await });
        }
    }
    let mut passed = true;
    while let Some(joined) = set.join_next().await {
        passed &= joined.map_err(|error| format!("case task: {error}"))?;
    }
    let run = Arc::into_inner(run).ok_or("run is still shared")?;
    Ok((run, passed))
}

async fn provision(run: &mut Run) -> Result<(), String> {
    for image in [&run.pins.images.postgres, &run.pins.images.probe] {
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

pub(crate) fn nonce(size: usize) -> Result<String, String> {
    let mut bytes = vec![0; size];
    getrandom::fill(&mut bytes).map_err(|error| format!("nonce: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
