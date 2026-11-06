mod dns;
mod http;
mod tcp;

use dns::address;
use std::io;
use std::process::ExitCode;

struct Endpoint<'a>(&'a str);

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("wire fixture: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> io::Result<()> {
    let value = |index| args.get(index).map(String::as_str).unwrap_or("");
    match value(0) {
        "serve" => Endpoint("127.0.0.2:53").serve([8, 8, 8, 8]),
        "probe" => Endpoint("198.51.100.1:15353").probe("neutral.fixture", value(1)),
        "dns" => Endpoint(value(1)).serve(address(value(2))?),
        "query" => Endpoint(value(1)).probe(value(2), value(3)),
        "pressure" => Endpoint(value(1)).pressure(),
        "blackhole" => Endpoint(value(1)).blackhole(),
        "http" => Endpoint(value(1)).http(),
        "fetch" => Endpoint(value(1)).fetch(),
        "hold" => Endpoint(value(1)).hold(),
        "fixed" => Endpoint(value(1)).fixed(),
        "stream" => Endpoint(value(1)).stream(value(2), value(3)),
        "reuse" => Endpoint(value(1)).reuse(),
        "fallback" => Endpoint(value(1)).fallback(),
        _ => Err(io::Error::other("unknown fixture mode")),
    }
}
