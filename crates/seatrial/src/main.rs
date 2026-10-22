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
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = plumb::identity!("SEATRIAL") {
        eprintln!("seatrial: {error}");
        return ExitCode::from(2);
    }
    let Cli {
        command: Command::Run { pins, cases },
    } = Cli::parse();
    if let Err(error) = plumb::identity::ready() {
        eprintln!("seatrial: {error}");
        return ExitCode::from(2);
    }
    match seatrial::run(&pins, &cases).await {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("seatrial: {error}");
            ExitCode::from(2)
        }
    }
}
