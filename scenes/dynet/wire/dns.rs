use super::{Endpoint, tcp};
use std::io::{self, ErrorKind};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::time::{Duration, Instant};

pub(super) fn address(value: &str) -> io::Result<[u8; 4]> {
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

pub(super) fn positive(body: &[u8], answer: &[u8]) -> bool {
    body[3] & 15 == 0 && body[6..8] != [0, 0] && body.ends_with(answer)
}

pub(super) fn ask(name: &str, id: u16) -> io::Result<Vec<u8>> {
    let mark = id.to_be_bytes();
    let mut body = vec![mark[0], mark[1], 1, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in name.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(io::Error::other("invalid fixture name"));
        }
        body.push(label.len() as u8);
        body.extend_from_slice(label.as_bytes());
    }
    body.extend_from_slice(&[0, 0, 1, 0, 1]);
    Ok(body)
}

fn response(body: &[u8], answer: [u8; 4], stream: bool) -> io::Result<Vec<u8>> {
    let question = question(body).ok_or_else(|| io::Error::other("invalid DNS question"))?;
    let large = question.starts_with(b"\x05large\x07fixture\x00");
    let count: u16 = if large && !stream {
        0
    } else if large {
        128
    } else {
        1
    };
    let mut reply = vec![
        body[0],
        body[1],
        if large && !stream { 0x83 } else { 0x81 },
        0x80,
        0,
        1,
    ];
    reply.extend_from_slice(&count.to_be_bytes());
    reply.extend_from_slice(&[0, 0, 0, 0]);
    reply.extend_from_slice(question);
    for index in 0..count {
        reply.extend_from_slice(&[0xc0, 0x0c, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4]);
        let mut address = answer;
        address[3] = address[3].wrapping_add(index as u8);
        reply.extend_from_slice(&address);
    }
    Ok(reply)
}

fn serve(mut stream: TcpStream, answer: [u8; 4]) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    while let Some(body) = tcp::read(&mut stream)? {
        tcp::write(&mut stream, &response(&body, answer, true)?)?;
    }
    Ok(())
}

impl Endpoint<'_> {
    pub(super) fn serve(&self, answer: [u8; 4]) -> io::Result<()> {
        let socket = UdpSocket::bind(self.0)?;
        let listener = TcpListener::bind(self.0)?;
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                std::thread::spawn(move || {
                    let _ = serve(stream, answer);
                });
            }
        });
        let mut room = [0u8; 1500];
        loop {
            let (length, peer) = socket.recv_from(&mut room)?;
            let body = &room[..length];
            let reply = response(body, answer, false)?;
            socket.send_to(&reply, peer)?;
        }
    }

    pub(super) fn query(&self, name: &str, port: u16) -> io::Result<Vec<u8>> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        socket.set_read_timeout(Some(Duration::from_millis(800)))?;
        socket.connect(self.0)?;
        let body = ask(name, 0x4123)?;
        socket.send(&body)?;
        let mut room = [0u8; 1500];
        let length = socket.recv(&mut room)?;
        if length < 12 || room[..2] != [0x41, 0x23] {
            return Err(io::Error::other("invalid DNS reply"));
        }
        Ok(room[..length].to_vec())
    }

    pub(super) fn probe(&self, name: &str, expected: &str) -> io::Result<()> {
        let began = Instant::now();
        let closed = expected == "closed";
        let answer = match expected {
            "open" => [8, 8, 8, 8],
            "closed" => [0; 4],
            _ => address(expected)?,
        };
        match self.query(name, 0) {
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

    pub(super) fn fixed(&self) -> io::Result<()> {
        let body = self.query("fixed.fixture", 40053)?;
        if !positive(&body, &[203, 0, 113, 7]) {
            return Err(io::Error::other("fixed-port DNS response disagrees"));
        }
        println!("fixed-port DNS positive");
        Ok(())
    }

    pub(super) fn pressure(&self) -> io::Result<()> {
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

    pub(super) fn pulse(&self, worker: usize, began: Instant) -> io::Result<()> {
        let mut serial = 0;
        while began.elapsed() < Duration::from_secs(8) {
            self.probe(&format!("p-{worker}-{serial}.protected.fixture"), "closed")?;
            serial += 1;
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }
}
