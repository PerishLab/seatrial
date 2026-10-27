mod client;

pub use client::Client;
use seatrial_lib::docker::Lane;
use seatrial_lib::probe::Ask;
pub use seatrial_lib::probe::start;

pub struct Reply {
    pub status: u16,
    pub body: String,
}

pub async fn send(lane: &Lane, ask: Ask<'_>) -> Result<Reply, String> {
    let reply = seatrial_lib::probe::send(lane, ask).await?;
    Ok(Reply {
        status: reply.status,
        body: reply.body,
    })
}
