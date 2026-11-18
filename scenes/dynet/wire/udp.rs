use crate::Endpoint;
use std::io;
use std::net::UdpSocket;
use std::time::Duration;

const LIMIT: usize = 1500;
const PATIENCE: Duration = Duration::from_secs(2);

pub fn echo(address: &str) -> io::Result<()> {
    let socket = UdpSocket::bind(address)?;
    let mut packet = [0; LIMIT];
    loop {
        let (size, peer) = socket.recv_from(&mut packet)?;
        let mut reply = format!("{} {} ", peer.ip(), socket.local_addr()?.port()).into_bytes();
        reply.extend_from_slice(&packet[..size]);
        socket.send_to(&reply, peer)?;
        println!("{} {} {}", peer.ip(), socket.local_addr()?.port(), size);
    }
}

impl Endpoint<'_> {
    pub fn round(&self, remote: &str, origin: &str, span: &str) -> io::Result<()> {
        let socket = self.bind(remote)?;
        let port = socket.peer_addr()?.port();
        let size: usize = span.parse().map_err(io::Error::other)?;
        if size > LIMIT {
            return Err(io::Error::other("fixture payload exceeds its limit"));
        }
        let body: Vec<u8> = (0..size).map(|index| (index % 251) as u8).collect();
        socket.send(&body)?;
        let mut reply = [0; LIMIT + 64];
        let received = socket.recv(&mut reply)?;
        let header = format!("{origin} {port} ");
        if reply[..received].strip_prefix(header.as_bytes()) != Some(body.as_slice()) {
            return Err(io::Error::other(
                "unexpected UDP payload, target, or egress",
            ));
        }
        println!("UDP round trip via {origin} to {remote}, {size} bytes");
        Ok(())
    }

    pub fn double(&self, first: &str, second: &str, origin: &str) -> io::Result<()> {
        let socket = UdpSocket::bind(self.0)?;
        socket.set_read_timeout(Some(PATIENCE))?;
        let first = first
            .parse::<std::net::SocketAddr>()
            .map_err(io::Error::other)?;
        let second = second
            .parse::<std::net::SocketAddr>()
            .map_err(io::Error::other)?;
        let targets = [(first, b"first".as_slice()), (second, b"second".as_slice())];
        for (target, body) in targets {
            socket.send_to(body, target)?;
        }
        let mut seen = [false; 2];
        let mut reply = [0; LIMIT];
        for _ in targets {
            let (size, source) = socket.recv_from(&mut reply)?;
            let index = targets
                .iter()
                .position(|(target, _)| *target == source)
                .ok_or_else(|| io::Error::other("unexpected UDP source"))?;
            let header = format!("{origin} {} ", source.port());
            if seen[index]
                || reply[..size].strip_prefix(header.as_bytes()) != Some(targets[index].1)
            {
                return Err(io::Error::other(
                    "UDP target association crossed or repeated",
                ));
            }
            seen[index] = true;
        }
        println!("Both targets replied through the exit on the same client socket.");
        Ok(())
    }

    pub fn quiet(&self, remote: &str) -> io::Result<()> {
        let socket = self.bind(remote)?;
        socket.send(b"protected UDP must not reach this destination")?;
        let mut room = [0; LIMIT];
        match socket.recv(&mut room) {
            Ok(_) => Err(io::Error::other("unexpected UDP reply during failure")),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::ConnectionRefused
                ) =>
            {
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    fn bind(&self, remote: &str) -> io::Result<UdpSocket> {
        let socket = UdpSocket::bind(self.0)?;
        socket.set_read_timeout(Some(PATIENCE))?;
        socket.connect(remote)?;
        Ok(socket)
    }
}
