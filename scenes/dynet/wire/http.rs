use super::Endpoint;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

const KEEP: &[u8] =
    b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: keep-alive\r\n\r\n";

impl Endpoint<'_> {
    pub(super) fn blackhole(&self) -> io::Result<()> {
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

    pub(super) fn http(&self) -> io::Result<()> {
        for stream in TcpListener::bind(self.0)?.incoming() {
            let stream = stream?;
            std::thread::spawn(move || {
                if let Err(error) = respond(stream) {
                    eprintln!("HTTP fixture: {error}");
                }
            });
        }
        Ok(())
    }

    pub(super) fn hold(&self) -> io::Result<()> {
        let mut stream = TcpStream::connect_timeout(
            &self.0.parse().map_err(io::Error::other)?,
            Duration::from_secs(2),
        )?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        let began = Instant::now();
        let mut room = [0; KEEP.len()];
        while began.elapsed() < Duration::from_secs(90) {
            stream.write_all(
                b"GET / HTTP/1.1\r\nHost: neutral.fixture\r\nConnection: keep-alive\r\n\r\n",
            )?;
            stream.read_exact(&mut room)?;
            if room != KEEP {
                return Err(io::Error::other("held HTTP response disagrees"));
            }
            println!("held {}ms", began.elapsed().as_millis());
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    pub(super) fn fetch(&self) -> io::Result<()> {
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

fn request(stream: &mut TcpStream) -> io::Result<Option<bool>> {
    let mut body = Vec::new();
    while !body.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        if stream.read(&mut byte)? == 0 {
            return Ok(None);
        }
        body.push(byte[0]);
        if body.len() > 1024 {
            return Err(io::Error::other("HTTP fixture request too large"));
        }
    }
    let wanted = b"Connection: keep-alive";
    Ok(Some(body.windows(wanted.len()).any(|part| part == wanted)))
}

fn respond(mut stream: TcpStream) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    while let Some(keep) = request(&mut stream)? {
        let reply = match keep {
            true => KEEP,
            false => b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        };
        stream.write_all(reply)?;
        if !keep {
            return Ok(());
        }
    }
    Ok(())
}
