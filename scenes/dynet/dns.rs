use std::io::{self, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::process::ExitCode;
use std::time::{Duration, Instant};

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
        _ => Err(io::Error::other("unknown fixture mode")),
    }
}

fn address(value: &str) -> io::Result<[u8; 4]> {
    value
        .parse::<std::net::Ipv4Addr>()
        .map(|ip| ip.octets())
        .map_err(io::Error::other)
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

fn positive(body: &[u8], answer: &[u8]) -> bool {
    body[3] & 15 == 0 && body[6..8] != [0, 0] && body.ends_with(answer)
}

struct Endpoint<'a>(&'a str);

impl Endpoint<'_> {
    fn serve(&self, answer: [u8; 4]) -> io::Result<()> {
        let socket = UdpSocket::bind(self.0)?;
        let mut room = [0u8; 1500];
        loop {
            let (length, peer) = socket.recv_from(&mut room)?;
            let body = &room[..length];
            let Some(question) = question(body) else {
                continue;
            };
            let mut reply = vec![body[0], body[1], 0x81, 0x80, 0, 1, 0, 1, 0, 0, 0, 0];
            reply.extend_from_slice(question);
            reply.extend_from_slice(&[0xc0, 0x0c, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4]);
            reply.extend_from_slice(&answer);
            socket.send_to(&reply, peer)?;
        }
    }

    fn query(&self, name: &str) -> io::Result<Vec<u8>> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(Duration::from_millis(800)))?;
        socket.connect(self.0)?;
        let mut body = vec![0x41, 0x23, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0];
        for label in name.split('.') {
            if label.is_empty() || label.len() > 63 {
                return Err(io::Error::other("invalid fixture name"));
            }
            body.push(label.len() as u8);
            body.extend_from_slice(label.as_bytes());
        }
        body.extend_from_slice(&[0, 0, 1, 0, 1]);
        socket.send(&body)?;
        let mut room = [0u8; 1500];
        let length = socket.recv(&mut room)?;
        if length < 12 || room[..2] != [0x41, 0x23] {
            return Err(io::Error::other("invalid DNS reply"));
        }
        Ok(room[..length].to_vec())
    }

    fn probe(&self, name: &str, expected: &str) -> io::Result<()> {
        let began = Instant::now();
        let closed = expected == "closed";
        let answer = match expected {
            "open" => [8, 8, 8, 8],
            "closed" => [0; 4],
            _ => address(expected)?,
        };
        match self.query(name) {
            Ok(body) if closed && (body[3] & 15 != 0 || body[6..8] == [0, 0]) => (),
            Err(error)
                if closed
                    && matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Ok(body) if !closed && positive(&body, &answer) => (),
            _ => return Err(io::Error::other(format!("DNS disagrees with {expected}"))),
        }
        if !closed {
            println!("dns {}us", began.elapsed().as_micros());
        }
        Ok(())
    }

    fn pressure(&self) -> io::Result<()> {
        let began = Instant::now();
        std::thread::scope(|scope| {
            let mut held = Vec::new();
            for worker in 0..64 {
                held.push(scope.spawn(move || self.pulse(worker, began)));
            }
            for handle in held {
                handle
                    .join()
                    .map_err(|_| io::Error::other("pressure thread failed"))??;
            }
            Ok(())
        })
    }

    fn pulse(&self, worker: usize, began: Instant) -> io::Result<()> {
        let mut serial = 0;
        while began.elapsed() < Duration::from_secs(8) {
            self.probe(&format!("p-{worker}-{serial}.protected.fixture"), "closed")?;
            serial += 1;
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }

    fn blackhole(&self) -> io::Result<()> {
        for stream in TcpListener::bind(self.0)?.incoming() {
            let stream = stream?;
            println!("accepted silent-node connection");
            std::thread::spawn(move || {
                let _held = stream;
                std::thread::sleep(Duration::from_secs(30));
            });
        }
        Ok(())
    }

    fn http(&self) -> io::Result<()> {
        for stream in TcpListener::bind(self.0)?.incoming() {
            let mut stream = stream?;
            stream.set_read_timeout(Some(Duration::from_secs(2)))?;
            let mut room = [0u8; 1024];
            if stream.read(&mut room)? == 0 {
                continue;
            }
            stream.write_all(
                b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )?;
        }
        Ok(())
    }

    fn fetch(&self) -> io::Result<()> {
        let began = Instant::now();
        let mut stream = TcpStream::connect_timeout(
            &self.0.parse().map_err(io::Error::other)?,
            Duration::from_secs(2),
        )?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: neutral.fixture\r\nConnection: close\r\n\r\n")?;
        let mut body = String::new();
        stream.take(4097).read_to_string(&mut body)?;
        if !body.starts_with("HTTP/1.1 204 ") || body.len() > 4096 {
            return Err(io::Error::other("HTTP control did not answer 204"));
        }
        println!("http {}us", began.elapsed().as_micros());
        Ok(())
    }
}
