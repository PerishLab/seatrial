use super::site::Run;
use seatrial_lib::binary::{Product, Release};

pub async fn keel(run: &Run) -> Result<Release, String> {
    let release = seatrial_lib::binary::fetch(
        &run.lane,
        Product {
            name: "keel",
            channel: &run.pins.keel.channel,
            pointer: run.pins.keel.pointer(),
        },
        &run.keel,
    )
    .await?;
    let model =
        std::fs::read(&run.source).map_err(|error| format!("{}: {error}", run.source.display()))?;
    seatrial_lib::file::write(&run.model, &model, 0o644)?;
    Ok(release)
}
