use crate::docker::{Lane, Spec};
use crate::docker::{Request, words};

pub const NAME: &str = "probe";

pub struct Reply {
    pub status: u16,
    pub body: String,
}

pub struct Ask<'a> {
    pub method: &'a str,
    pub url: String,
    pub auth: Option<String>,
    pub body: Option<String>,
}

pub async fn start(lane: &Lane, image: &str) -> Result<(), String> {
    lane.start(&Spec {
        name: lane.host(NAME),
        image: image.into(),
        entry: Some("sleep".into()),
        args: words(&["infinity"]),
        ..Spec::default()
    })
    .await
}

pub async fn fetch(lane: &Lane, url: &str) -> Result<Vec<u8>, String> {
    let host = lane.host(NAME);
    let args = words(&["exec", &host, "curl", "-fsSL", "--max-time", "300", url]);
    download(
        Request {
            args: &args,
            stop: lane.stop.clone(),
        },
        url,
    )
    .await
}

pub struct Download<'a> {
    pub lane: &'a Lane,
    pub url: &'a str,
    pub limit: u64,
}

impl Download<'_> {
    pub async fn run(&self) -> Result<Vec<u8>, String> {
        let lane = self.lane;
        let url = self.url;
        let limit = self.limit;
        if limit == 0 {
            return Err("artifact download needs a nonzero byte budget".into());
        }
        let host = lane.host(NAME);
        let version = Request {
            args: &words(&["exec", &host, "curl", "--version"]),
            stop: lane.stop.clone(),
        }
        .run()
        .await?;
        let text = version.text();
        let mut fields = text.split_whitespace();
        let name = fields.next();
        let mut parts = fields.next().unwrap_or("").split('.');
        let current = parts
            .next()
            .and_then(|part| part.parse::<u64>().ok())
            .zip(parts.next().and_then(|part| part.parse::<u64>().ok()));
        if name != Some("curl") || !version.ok || current.is_none_or(|value| value < (8, 4)) {
            return Err("bounded artifact download requires curl 8.4 or newer".into());
        }
        let args = words(&[
            "exec",
            &host,
            "curl",
            "-fsSL",
            "--max-time",
            "300",
            "--max-filesize",
            &limit.to_string(),
            url,
        ]);
        download(
            Request {
                args: &args,
                stop: lane.stop.clone(),
            },
            url,
        )
        .await
    }
}

async fn download(request: Request<'_>, url: &str) -> Result<Vec<u8>, String> {
    request
        .run()
        .await
        .and_then(|output| match output.ok {
            true => Ok(output.out),
            false => Err(output.err),
        })
        .map_err(|error| format!("fetch {url}: {error}"))
}

pub async fn send(lane: &Lane, ask: Ask<'_>) -> Result<Reply, String> {
    let host = lane.host(NAME);
    let mut args = words(&["exec", &host, "curl", "-sS", "--max-time", "10"]);
    args.extend(words(&[
        "--request",
        ask.method,
        "--write-out",
        "\n%{http_code}",
    ]));
    if let Some(auth) = ask.auth {
        args.extend(["--header".into(), format!("authorization: {auth}")]);
    }
    if let Some(body) = ask.body {
        args.extend(words(&["--header", "content-type: application/json"]));
        args.extend(["--data".into(), body]);
    }
    args.push(ask.url);
    let output = Request {
        args: &args,
        stop: lane.stop.clone(),
    }
    .run()
    .await?;
    if !output.ok {
        return Ok(Reply {
            status: 0,
            body: output.err,
        });
    }
    let text = String::from_utf8_lossy(&output.out).into_owned();
    let (body, code) = text.rsplit_once('\n').unwrap_or(("", text.as_str()));
    let status = code
        .trim()
        .parse()
        .map_err(|_| format!("probe: no status in reply {code:?}"))?;
    Ok(Reply {
        status,
        body: body.to_string(),
    })
}
