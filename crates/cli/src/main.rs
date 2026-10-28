mod pin;
mod record;
mod scene;
mod source;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "seatrial", version = plumb::version!("SEATRIAL"))]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Run(Input),
}

#[derive(clap::Args)]
struct Input {
    #[arg(long, default_value = "seatrial.toml")]
    pins: PathBuf,
    #[arg(long = "case")]
    cases: Vec<String>,
    #[arg(long)]
    scenes: Option<PathBuf>,
    #[arg(long, value_name = "NEW_FILE")]
    report: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = plumb::identity!("SEATRIAL") {
        eprintln!("seatrial: {error}");
        return ExitCode::from(2);
    }
    let Cli {
        command: Command::Run(input),
    } = Cli::parse();
    if let Err(error) = plumb::identity::ready() {
        eprintln!("seatrial: {error}");
        return ExitCode::from(2);
    }
    let mut record = match record::Record::open(input.report.as_deref()) {
        Ok(record) => record,
        Err(error) => {
            eprintln!("seatrial: {error}");
            return ExitCode::from(2);
        }
    };
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
    let result = run(&input, stop.clone(), &mut record).await;
    listener.abort();
    record.settle(&result, *stop.borrow());
    if let Err(error) = record.write() {
        eprintln!("seatrial: {error}");
        return ExitCode::from(2);
    }
    if *stop.borrow() {
        return ExitCode::from(130);
    }
    match result {
        Ok(true) if record.ok => ExitCode::SUCCESS,
        Ok(true) => ExitCode::from(2),
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("seatrial: {error}");
            ExitCode::from(2)
        }
    }
}

async fn run(
    input: &Input,
    stop: seatrial_lib::binary::Stop,
    record: &mut record::Record,
) -> Result<bool, String> {
    let pins = pin::read(&input.pins)?;
    let root = source::prepare(&pins, input.scenes.as_deref())?;
    record.scene = root.depot.as_ref().map(|material| material.pointer.clone());
    if *stop.borrow() {
        return Err("cancelled before execution".into());
    }
    scene::run(
        scene::Request {
            pins: &pins,
            root: &root.path,
            names: &input.cases,
        },
        stop,
        record,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version() {
        let error = Cli::try_parse_from(["seatrial", "--version"])
            .err()
            .expect("version must use the display exit");
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayVersion);
        assert_eq!(error.exit_code(), 0);
        assert_eq!(
            error.to_string(),
            format!("seatrial {}\n", plumb::version!("SEATRIAL"))
        );
    }
}
