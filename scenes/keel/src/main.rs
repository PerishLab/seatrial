mod case;
mod config;
mod pg;
mod probe;
mod run;
mod site;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use config::{Images, Pins, State};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Prepare(Input),
    Run {
        state: PathBuf,
        case: String,
        #[arg(required = true)]
        stores: Vec<String>,
    },
}

#[derive(Args)]
struct Input {
    ns: String,
    dir: PathBuf,
    keel: PathBuf,
    model: PathBuf,
    image: String,
    version: String,
    postgres: String,
    sqlite: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Prepare(input) => prepare(input).await,
        Command::Run {
            state,
            case,
            stores,
        } => run::cases(&state, &case, stores).await,
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("keel scene: {error}");
            ExitCode::from(2)
        }
    }
}

async fn prepare(input: Input) -> Result<bool, String> {
    let model = input.dir.join("model.json");
    let bytes = std::fs::read(input.model).map_err(|error| error.to_string())?;
    seatrial_lib::file::write(&model, &bytes, 0o644)?;
    let state = State {
        ns: input.ns,
        dir: input.dir,
        keel: input.keel,
        model,
        image: format!("{}:{}", input.image, input.version.trim_start_matches('v')),
        secret: seatrial_lib::file::nonce(16)?,
        pins: Pins {
            images: Images {
                postgres: input.postgres,
                sqlite: input.sqlite,
            },
        },
    };
    state.write()?;
    run::prepare(&state.run()).await?;
    Ok(true)
}
