use crate::docker::{call, must, words};
use crate::lane::{Lane, Spec};

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
    must(&args)
        .await
        .map(|output| output.out)
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
    let output = call(&args).await?;
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
