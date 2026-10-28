use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use plumb::depot::{sha, v3};
use seatrial_lib::depot::Source;

struct Host {
    url: String,
    pointer: v3::Pointer,
    responses: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    stop: Arc<AtomicBool>,
    task: Option<JoinHandle<()>>,
}

impl Host {
    fn new(files: &[(&str, &[u8])]) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let objects = files
            .iter()
            .map(|(path, body)| v3::Object {
                path: (*path).into(),
                sha256: sha(body),
                size: body.len() as u64,
                media: "application/octet-stream".into(),
                executable: path.ends_with(".sh"),
            })
            .collect();
        let manifest = v3::Manifest::new(
            v3::Identity {
                product: "seatrial".into(),
                channel: "stable".into(),
                version: "v1.0.0".into(),
                marker: v3::Marker {
                    name: "v1.0.0".into(),
                    sha256: "a".repeat(64),
                },
                kind: v3::Kind::Scene,
            },
            objects,
        )
        .unwrap();
        let pointer = v3::Pointer::new(
            &manifest,
            v3::Publication {
                source: &url,
                prior: None,
                created: "2026-10-28T00:00:00Z".into(),
            },
        )
        .unwrap();
        let prefix = pointer.manifest.url.strip_prefix(&url).unwrap();
        let base = prefix.strip_suffix(v3::LEAF).unwrap();
        let mut responses = BTreeMap::from([
            (
                "/channels/stable/scenes/latest.json".into(),
                pointer.encode().unwrap(),
            ),
            (
                "/channels/stable/scenes/versions/v1.0.0/latest.json".into(),
                pointer.encode().unwrap(),
            ),
            (prefix.into(), manifest.encode().unwrap()),
        ]);
        for (path, body) in files {
            responses.insert(format!("{base}objects/{path}"), body.to_vec());
        }
        let responses = Arc::new(Mutex::new(responses));
        let stop = Arc::new(AtomicBool::new(false));
        let (served, ended) = (responses.clone(), stop.clone());
        let task = std::thread::spawn(move || serve(listener, served, ended));
        Self {
            url,
            pointer,
            responses,
            stop,
            task: Some(task),
        }
    }

    fn source(&self) -> Source {
        Source {
            authority: self.url.clone(),
            product: "seatrial".into(),
            channel: "stable".into(),
        }
    }

    fn object(&self, path: &str) -> String {
        let prefix = self.pointer.manifest.url.strip_prefix(&self.url).unwrap();
        format!("{}objects/{path}", prefix.strip_suffix(v3::LEAF).unwrap())
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(task) = self.task.take() {
            task.join().unwrap();
        }
    }
}

fn serve(
    listener: TcpListener,
    responses: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Relaxed) {
        let (mut stream, _) = match listener.accept() {
            Ok(stream) => stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            Err(error) => panic!("listener: {error}"),
        };
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        stream
            .set_write_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let mut request = [0; 8192];
        let Ok(size) = stream.read(&mut request) else {
            continue;
        };
        let request = String::from_utf8_lossy(&request[..size]);
        let path = request.split_whitespace().nth(1).unwrap_or("");
        let body = responses.lock().unwrap().get(path).cloned();
        let status = if body.is_some() {
            "200 OK"
        } else {
            "404 Not Found"
        };
        let body = body.unwrap_or_default();
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream
            .write_all(head.as_bytes())
            .and_then(|_| stream.write_all(&body));
    }
}

#[test]
fn material() {
    use std::os::unix::fs::PermissionsExt;
    let host = Host::new(&[
        ("manifest.json", b"ordinary asset"),
        ("run.sh", b"exit 91\n"),
    ]);
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("material");
    let found = host.source().fetch(&path).unwrap();
    assert_eq!(found.pointer, host.pointer);
    assert_eq!(
        std::fs::read(path.join("manifest.json")).unwrap(),
        b"ordinary asset"
    );
    assert_eq!(
        std::fs::metadata(path.join("run.sh"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    assert_eq!(found.manifest.objects.len(), 2);
    assert!(host.source().fetch(&path).is_err());
    assert_eq!(
        std::fs::read(path.join("manifest.json")).unwrap(),
        b"ordinary asset"
    );
}

#[test]
fn drift() {
    let host = Host::new(&[("asset", b"expected")]);
    let root = tempfile::tempdir().unwrap();
    let mut foreign = host.source();
    foreign.product = "other".into();
    assert!(
        foreign
            .fetch(&root.path().join("foreign"))
            .unwrap_err()
            .contains("projection")
    );
    host.responses
        .lock()
        .unwrap()
        .insert(host.object("asset"), b"changed".to_vec());
    assert!(
        host.source()
            .fetch(&root.path().join("drift"))
            .unwrap_err()
            .contains("drift")
    );
    let mut pointer = host.pointer.clone();
    pointer.prior = Some("b".repeat(64));
    host.responses.lock().unwrap().insert(
        "/channels/stable/scenes/latest.json".into(),
        pointer.encode().unwrap(),
    );
    assert!(
        host.source()
            .fetch(&root.path().join("standing"))
            .unwrap_err()
            .contains("standing")
    );
}

#[test]
fn paths() {
    let host = Host::new(&[("folder//asset", b"bytes")]);
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("material");
    assert!(host.source().fetch(&path).is_err());
    assert!(!path.exists());
    let mut source = host.source();
    source.channel = "../stable".into();
    assert!(source.fetch(&path).is_err());
}
