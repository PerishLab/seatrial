use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use seatrial_lib::docker::{must, words};
use tokio::task::JoinSet;

use crate::config::State;
use crate::site::{Run, Site, Store};

pub async fn prepare(run: &Run) -> Result<(), String> {
    for image in [
        &run.pins.images.postgres,
        &run.pins.images.sqlite,
        &run.image,
    ] {
        must(&words(&["pull", "--quiet", image])).await?;
    }
    crate::pg::start(&run.lane, &run.pins.images.postgres, &run.secret).await?;
    let digest = must(&words(&[
        "image",
        "inspect",
        "--format",
        "{{index .RepoDigests 0}}",
        &run.image,
    ]))
    .await?;
    println!("image   {}", digest.text());
    Ok(())
}

pub async fn cases(path: &Path, name: &str, stores: Vec<String>) -> Result<bool, String> {
    let case = crate::case::known(name).ok_or_else(|| format!("unknown Keel case {name}"))?;
    let stores = stores
        .iter()
        .map(|store| match store.as_str() {
            "lite" => Ok(Store::Lite),
            "pg" => Ok(Store::Pg),
            _ => Err(format!("unknown store {store}")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let run = Arc::new(State::read(path)?.run());
    let mut set = JoinSet::new();
    for store in stores {
        let run = Arc::clone(&run);
        set.spawn(async move { judge(&run, case, store).await });
    }
    let mut passed = true;
    while let Some(result) = set.join_next().await {
        match result {
            Ok(result) => passed &= result,
            Err(error) => {
                set.shutdown().await;
                return Err(format!("case task: {error}"));
            }
        }
    }
    Ok(passed)
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
