use clap::Parser;
use std::io::{self, ErrorKind};
use std::net::UdpSocket;
use std::process::ExitCode;
use std::time::Duration;

#[derive(Parser)]
struct Input {
    mode: String,
    expected: Option<String>,
}

fn main() -> ExitCode {
    let input = Input::parse();
    let result = match input.mode.as_str() {
        "serve" => serve(),
        "probe" => probe(input.expected.as_deref() == Some("closed")),
        _ => Err(io::Error::other("unknown DNS fixture mode")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("DNS fixture: {error}");
            ExitCode::FAILURE
        }
    }
}

fn serve() -> io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.2:53")?;
    let mut room = [0u8; 1500];
    loop {
        let (length, peer) = socket.recv_from(&mut room)?;
        let body = &room[..length];
        let Some(question) = question(body) else {
            continue;
        };
        let mut reply = vec![body[0], body[1], 0x81, 0x80, 0, 1, 0, 1, 0, 0, 0, 0];
        reply.extend_from_slice(question);
        reply.extend_from_slice(&[0xc0, 0x0c, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4, 8, 8, 8, 8]);
        socket.send_to(&reply, peer)?;
    }
}

fn question(body: &[u8]) -> Option<&[u8]> {
    if body.len() < 17 {
        return None;
    }
    let mut end = 12;
    loop {
        let length = usize::from(*body.get(end)?);
        if length == 0 {
            return body.get(12..end + 5);
        }
        if length > 63 {
            return None;
        }
        end += length + 1;
    }
}

fn probe(closed: bool) -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_millis(800)))?;
    socket.connect("198.51.100.1:15353")?;
    let mut query = vec![0x41, 0x23, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in ["neutral", "fixture"] {
        query.push(label.len() as u8);
        query.extend_from_slice(label.as_bytes());
    }
    query.extend_from_slice(&[0, 0, 1, 0, 1]);
    socket.send(&query)?;
    let mut room = [0u8; 1500];
    match socket.recv(&mut room) {
        Ok(length) if !closed && length >= 16 => {
            if room[..2] != [0x41, 0x23] || room[length - 4..length] != [8, 8, 8, 8] {
                return Err(io::Error::other("unexpected DNS reply"));
            }
            Ok(())
        }
        Err(error)
            if closed && matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
        {
            Ok(())
        }
        observed => Err(io::Error::other(format!(
            "DNS availability disagrees with closed={closed}: {observed:?}"
        ))),
    }
}
