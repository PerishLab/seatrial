use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::docker::{must, words};
use crate::lane::{Lane, Spec};
use crate::pin::Pins;
use crate::probe::{self, Ask, Reply};

const ESTATE: &str = "/keel/.local";
const WAIT: Duration = Duration::from_secs(60);

pub struct Run {
    pub lane: Lane,
    pub pins: Pins,
    pub secret: String,
    pub keel: PathBuf,
    pub model: PathBuf,
    pub source: PathBuf,
    pub image: String,
}

#[derive(Clone, Copy)]
pub enum Store {
    Lite,
    Pg,
}

impl Store {
    pub fn name(self) -> &'static str {
        match self {
            Self::Lite => "lite",
            Self::Pg => "pg",
        }
    }
}

pub struct Site<'a> {
    run: &'a Run,
    case: &'a str,
    store: Store,
    volume: String,
}

impl<'a> Site<'a> {
    pub async fn open(run: &'a Run, case: &'a str, store: Store) -> Result<Self, String> {
        let volume = run.lane.volume(&format!("{case}-{}", store.name())).await?;
        if let Store::Pg = store {
            crate::pg::create(run, case).await?;
        }
        Ok(Self {
            run,
            case,
            store,
            volume,
        })
    }

    fn component(&self) -> String {
        format!("keel-{}-{}", self.case, self.store.name())
    }

    fn url(&self) -> String {
        match self.store {
            Store::Lite => format!("sqlite:{ESTATE}/keel.db"),
            Store::Pg => crate::pg::url(self.run, self.case),
        }
    }

    pub fn base(&self) -> String {
        format!("http://{}:8080", self.run.lane.host(&self.component()))
    }

    fn spec(&self, entry: &str, args: Vec<String>) -> Spec {
        let keel = format!("{}:/opt/keel:ro", self.run.keel.display());
        let model = format!("{}:/opt/model.json:ro", self.run.model.display());
        Spec {
            image: self.run.image.clone(),
            entry: Some(entry.into()),
            env: vec![format!("KEEL_STORE={}", self.url())],
            mounts: vec![format!("{}:{ESTATE}", self.volume), keel, model],
            args,
            ..Spec::default()
        }
    }

    pub async fn keel(&self, args: &[&str]) -> Result<(bool, String), String> {
        let output = self
            .run
            .lane
            .once(&self.spec("/opt/keel", words(args)))
            .await?;
        let text = output.text();
        Ok((output.ok, if output.ok { text } else { output.err }))
    }

    pub async fn bootstrap(&self, custody: &str) -> Result<bool, String> {
        let path = format!("{ESTATE}/{custody}");
        let args = [
            "bootstrap",
            "--model",
            "/opt/model.json",
            "--custody",
            &path,
        ];
        Ok(self.keel(&args).await?.0)
    }

    pub async fn held(&self, custody: &str) -> Result<bool, String> {
        let path = format!("{ESTATE}/{custody}");
        let output = self
            .run
            .lane
            .once(&self.spec("test", words(&["-f", &path])))
            .await?;
        Ok(output.ok)
    }

    pub async fn token(&self, custody: &str) -> Result<String, String> {
        let path = format!("{ESTATE}/{custody}");
        let output = self
            .run
            .lane
            .once(&self.spec("cat", words(&[&path])))
            .await?;
        if !output.ok {
            return Err(format!("custody {custody}: {}", output.err));
        }
        Ok(output.text())
    }

    pub async fn serve(&self) -> Result<(), String> {
        let mut spec = self.spec("keel-api", Vec::new());
        spec.name = self.run.lane.host(&self.component());
        spec.mounts.truncate(1);
        self.run.lane.start(&spec).await?;
        self.ready().await
    }

    pub async fn restart(&self) -> Result<(), String> {
        let name = self.run.lane.host(&self.component());
        must(&words(&["restart", &name])).await?;
        self.ready().await
    }

    async fn ready(&self) -> Result<(), String> {
        let start = Instant::now();
        while start.elapsed() < WAIT {
            if self.get("/health", None).await?.status == 200 {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Err(format!("{} never became healthy", self.component()))
    }

    pub async fn get(&self, path: &str, auth: Option<String>) -> Result<Reply, String> {
        let url = format!("{}{path}", self.base());
        let ask = Ask {
            method: "GET",
            url,
            auth,
            body: None,
        };
        probe::send(&self.run.lane, ask).await
    }

    pub async fn post(
        &self,
        path: &str,
        auth: Option<String>,
        body: String,
    ) -> Result<Reply, String> {
        let url = format!("{}{path}", self.base());
        let ask = Ask {
            method: "POST",
            url,
            auth,
            body: Some(body),
        };
        probe::send(&self.run.lane, ask).await
    }
}
