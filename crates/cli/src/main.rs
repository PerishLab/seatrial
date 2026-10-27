mod pin;
mod scene;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "seatrial")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Run {
        #[arg(long, default_value = "seatrial.toml")]
        pins: PathBuf,
        #[arg(long = "case")]
        cases: Vec<String>,
        #[arg(long, default_value = "scenes")]
        scenes: PathBuf,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = plumb::identity!("SEATRIAL") {
        eprintln!("seatrial: {error}");
        return ExitCode::from(2);
    }
    let Cli {
        command: Command::Run {
            pins,
            cases,
            scenes,
        },
    } = Cli::parse();
    if let Err(error) = plumb::identity::ready() {
        eprintln!("seatrial: {error}");
        return ExitCode::from(2);
    }
    let (sender, stop) = tokio::sync::watch::channel(false);
    let mut terminate =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(error) => {
                eprintln!("seatrial: signal listener: {error}");
                return ExitCode::from(2);
            }
        };
    let listener = tokio::spawn(async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
        let _ = sender.send(true);
    });
    let result = run(&pins, &cases, &scenes, stop.clone()).await;
    listener.abort();
    if *stop.borrow() {
        return ExitCode::from(130);
    }
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("seatrial: {error}");
            ExitCode::from(2)
        }
    }
}

async fn run(
    path: &std::path::Path,
    cases: &[String],
    scenes: &std::path::Path,
    stop: seatrial_lib::binary::Stop,
) -> Result<bool, String> {
    let pins = pin::read(path)?;
    scene::run(&pins, scenes, cases, stop).await
}
