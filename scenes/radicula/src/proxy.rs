use std::{
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    time::Duration,
};

pub fn serve(address: &str, upstream: &str, fault: &str) {
    let listener = TcpListener::bind(address).unwrap();
    let fault = Arc::new(Mutex::new(fault.to_owned()));
    for socket in listener.incoming() {
        let upstream = upstream.to_owned();
        let fault = fault.clone();
        std::thread::spawn(move || {
            let _ = forward(socket.unwrap(), &upstream, &fault);
        });
    }
}

fn forward(mut client: TcpStream, upstream: &str, fault: &Mutex<String>) -> io::Result<()> {
    client.set_read_timeout(Some(Duration::from_secs(3)))?;
    client.set_write_timeout(Some(Duration::from_secs(3)))?;
    let command = frame(&mut BufReader::new(client.try_clone()?), 0)?;
    let mut server = TcpStream::connect(upstream)?;
    server.set_read_timeout(Some(Duration::from_secs(3)))?;
    server.set_write_timeout(Some(Duration::from_secs(3)))?;
    server.write_all(&command)?;
    let reply = frame(&mut BufReader::new(server), 0)?;
    let path = fault.lock().unwrap();
    let armed = fs::read_to_string(&*path).unwrap_or_default();
    let script = String::from_utf8_lossy(&command);
    let drop = match armed.trim() {
        "acquire" => script.contains("local policy"),
        "release" => script.contains("redis.call('ZREM',"),
        "renew" => script.contains("redis.call('ZADD',") && !script.contains("local policy"),
        _ => false,
    };
    if drop {
        fs::remove_file(&*path)?;
        fs::write(format!("{path}.observed"), &armed)?;
        return Ok(());
    }
    client.write_all(&reply)
}

fn frame(reader: &mut impl BufRead, depth: usize) -> io::Result<Vec<u8>> {
    if depth > 8 {
        return Err(io::Error::other("fixture nesting limit"));
    }
    let mut line = Vec::new();
    reader.take(131072).read_until(b'\n', &mut line)?;
    if !line.ends_with(b"\r\n") {
        return Err(io::Error::other("incomplete fixture frame"));
    }
    let kind = *line
        .first()
        .ok_or_else(|| io::Error::other("empty frame"))?;
    let number = || {
        std::str::from_utf8(&line[1..line.len() - 2])
            .unwrap()
            .parse::<i64>()
            .unwrap()
    };
    match kind {
        b'$' => {
            let length = number();
            bulk(reader, &mut line, length)?;
        }
        b'*' => {
            let count = number();
            if count > 256 {
                return Err(io::Error::other("fixture array limit"));
            }
            for _ in 0..count {
                line.extend(frame(reader, depth + 1)?);
            }
        }
        _ => {}
    }
    Ok(line)
}

fn bulk(reader: &mut impl BufRead, line: &mut Vec<u8>, length: i64) -> io::Result<()> {
    if length < 0 {
        return Ok(());
    }
    let size = usize::try_from(length).unwrap();
    if size > 131072 {
        return Err(io::Error::other("fixture bulk limit"));
    }
    let mut body = vec![0; size + 2];
    reader.read_exact(&mut body)?;
    line.extend_from_slice(&body);
    Ok(())
}
