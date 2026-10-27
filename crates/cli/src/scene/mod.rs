mod manifest;
mod stage;

use std::path::{Path, PathBuf};
use std::time::Instant;

use seatrial_lib::binary::{self, Stop};
use seatrial_lib::docker::Lane;

use crate::pin::Pins;

pub async fn run(pins: &Pins, root: &Path, name: &str, stop: Stop) -> Result<bool, String> {
    let scene = manifest::load(root, name)?;
    let lane = Lane::open(PathBuf::from(".local"), stop).await?;
    println!("lane    {} (perish.owner=seatrial)", lane.ns);
    let start = Instant::now();
    let result = trial(pins, &scene, &lane).await;
    let result = lane.finish(result).await;
    match result {
        Ok(()) => {
            println!("pass    {name} {:.3}s", start.elapsed().as_secs_f64());
            Ok(true)
        }
        Err(error) => {
            eprintln!(
                "fail    {name} {:.3}s {error}",
                start.elapsed().as_secs_f64()
            );
            Ok(false)
        }
    }
}

async fn trial(pins: &Pins, scene: &manifest::Scene, lane: &Lane) -> Result<(), String> {
    let spec = stage::prepare(pins, scene, lane).await?;
    let output = binary::run(&spec, lane.stop.clone()).await?;
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        return Err(format!("scene command {}", output.status));
    }
    Ok(())
}
