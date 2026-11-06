use super::Endpoint;
use super::dns::{address, ask, positive};
use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub(super) fn read(stream: &mut TcpStream) -> io::Result<Option<Vec<u8>>> {
    let mut prefix = [0; 2];
    if stream.read(&mut prefix[..1])? == 0 {
        return Ok(None);
    }
    stream.read_exact(&mut prefix[1..])?;
    let mut body = vec![0; usize::from(u16::from_be_bytes(prefix))];
    if body.len() < 12 {
        return Err(io::Error::other("short TCP DNS frame"));
    }
    stream.read_exact(&mut body)?;
    Ok(Some(body))
}

pub(super) fn write(writer: &mut impl Write, body: &[u8]) -> io::Result<()> {
    let size = u16::try_from(body.len()).map_err(io::Error::other)?;
    let mut wire = size.to_be_bytes().to_vec();
    wire.extend_from_slice(body);
    writer.write_all(&wire)
}

impl Endpoint<'_> {
    fn connect(&self) -> io::Result<TcpStream> {
        let stream = TcpStream::connect_timeout(
            &self.0.parse().map_err(io::Error::other)?,
            Duration::from_secs(2),
        )?;
        stream.set_read_timeout(Some(Duration::from_millis(800)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        stream.set_nodelay(true)?;
        Ok(stream)
    }

    pub(super) fn stream(&self, name: &str, expected: &str) -> io::Result<()> {
        let mut stream = self.connect()?;
        write(&mut stream, &ask(name, 0x4123)?)?;
        let closed = expected == "closed";
        match read(&mut stream) {
            Ok(Some(body)) if body[..2] != [0x41, 0x23] => {
                return Err(io::Error::other("TCP DNS identity differs"));
            }
            Ok(Some(body)) if closed && (body[3] & 15 != 0 || body[6..8] == [0, 0]) => (),
            Ok(None) if closed => (),
            Err(error)
                if closed
                    && matches!(
                        error.kind(),
                        ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::UnexpectedEof
                    ) => {}
            Ok(Some(body)) if !closed && positive(&body, &address(expected)?) => (),
            _ => return Err(io::Error::other("TCP DNS result differs")),
        }
        println!("TCP DNS {expected}");
        Ok(())
    }

    pub(super) fn reuse(&self) -> io::Result<()> {
        let mut stream = self.connect()?;
        let mut requests = Vec::new();
        write(&mut requests, &ask("neutral.fixture", 0x4161)?)?;
        write(&mut requests, &ask("large.fixture", 0x4162)?)?;
        stream.write_all(&requests[..1])?;
        std::thread::sleep(Duration::from_millis(40));
        stream.write_all(&requests[1..])?;
        let first = read(&mut stream)?.ok_or_else(|| io::Error::other("first reply missing"))?;
        let second = read(&mut stream)?.ok_or_else(|| io::Error::other("second reply missing"))?;
        if first[..2] != [0x41, 0x61] || !positive(&first, &[203, 0, 113, 7]) {
            return Err(io::Error::other("first pipelined reply differs"));
        }
        if second[..2] != [0x41, 0x62] {
            return Err(io::Error::other("second pipelined identity differs"));
        }
        if second.len() <= 1500
            || second[6..8] != [0, 128]
            || !positive(&second, &[203, 0, 113, 134])
        {
            return Err(io::Error::other(
                "large pipelined reply was truncated or changed",
            ));
        }
        println!("fragmented TCP prefix, pipeline identities and full large response positive");
        Ok(())
    }

    pub(super) fn fallback(&self) -> io::Result<()> {
        let body = self.query("large.fixture", 0)?;
        if body[2] & 2 == 0 || body[6..8] != [0, 0] {
            return Err(io::Error::other("UDP truncation prerequisite missing"));
        }
        self.reuse()?;
        println!("UDP truncation followed by complete TCP retry positive");
        Ok(())
    }
}
