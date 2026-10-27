use std::time::{Duration, Instant};

use serde_json::Value;

use crate::config::State;
use crate::probe::Client;
use crate::probe::Reply;
use seatrial_lib::docker::{Lane, Spec};
use seatrial_lib::docker::{must, words};

const ESTATE: &str = "/keel/.local";
const WAIT: Duration = Duration::from_secs(60);

pub struct Run {
    pub lane: Lane,
    pub state: State,
}

impl std::ops::Deref for Run {
    type Target = State;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
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

    pub async fn propose(&self, model: &Value) -> Result<(bool, String), String> {
        let path = self
            .run
            .lane
            .dir
            .join(format!("{}-{}.json", self.case, self.store.name()));
        seatrial_lib::file::write(&path, model.to_string().as_bytes(), 0o644)?;
        let custody = format!("{ESTATE}/custody");
        let args = [
            "bootstrap",
            "--model",
            "/opt/model.json",
            "--custody",
            &custody,
        ];
        let mut spec = self.spec("/opt/keel", words(&args));
        spec.mounts[2] = format!("{}:/opt/model.json:ro", path.display());
        let output = self.run.lane.once(&spec).await?;
        let text = output.text();
        Ok((output.ok, if output.ok { text } else { output.err }))
    }

    pub fn model(&self) -> Result<Value, String> {
        let bytes = std::fs::read(&self.run.model)
            .map_err(|error| format!("{}: {error}", self.run.model.display()))?;
        serde_json::from_slice(&bytes).map_err(|error| format!("model: {error}"))
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

    pub fn client(&self, auth: Option<String>) -> Client<'_> {
        Client {
            lane: &self.run.lane,
            base: self.base(),
            auth,
        }
    }

    pub async fn serve(&self) -> Result<(), String> {
        self.launch(&self.component()).await?;
        self.ready(&self.client(None)).await
    }

    pub async fn twin(&self, auth: String) -> Result<Client<'_>, String> {
        let component = format!("{}-far", self.component());
        self.launch(&component).await?;
        let far = Client {
            lane: &self.run.lane,
            base: format!("http://{}:8080", self.run.lane.host(&component)),
            auth: Some(auth),
        };
        self.ready(&far).await?;
        Ok(far)
    }

    async fn launch(&self, component: &str) -> Result<(), String> {
        let mut spec = self.spec("keel-api", Vec::new());
        spec.name = self.run.lane.host(component);
        spec.mounts.truncate(1);
        self.run.lane.start(&spec).await
    }

    pub async fn restart(&self) -> Result<(), String> {
        let name = self.run.lane.host(&self.component());
        must(&words(&["restart", &name])).await?;
        self.ready(&self.client(None)).await
    }

    async fn ready(&self, client: &Client<'_>) -> Result<(), String> {
        let start = Instant::now();
        while start.elapsed() < WAIT {
            if client.get("/health").await?.status == 200 {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Err(format!("{} never became healthy", client.base))
    }

    pub async fn sql(&self, text: &str) -> Result<String, String> {
        let output = match self.store {
            Store::Pg => crate::pg::session(self.run, self.case, "seatrial", text).await?,
            Store::Lite => {
                let spec = Spec {
                    image: self.run.pins.images.sqlite.clone(),
                    entry: Some("sqlite3".into()),
                    user: Some("10001:10001".into()),
                    mounts: vec![format!("{}:{ESTATE}", self.volume)],
                    args: words(&[&format!("{ESTATE}/keel.db"), text]),
                    ..Spec::default()
                };
                self.run.lane.once(&spec).await?
            }
        };
        match output.ok {
            true => Ok(output.text()),
            false => Err(format!("{} sql: {}", self.store.name(), output.err)),
        }
    }

    pub async fn session(&self, app: &str, sql: &str) -> Result<(), String> {
        crate::pg::session(self.run, self.case, app, sql)
            .await
            .map(drop)
    }

    pub fn run(&self) -> &Run {
        self.run
    }

    pub fn case(&self) -> &str {
        self.case
    }

    pub fn store(&self) -> Store {
        self.store
    }

    pub async fn get(&self, path: &str, auth: Option<String>) -> Result<Reply, String> {
        self.client(auth).get(path).await
    }

    pub async fn post(
        &self,
        path: &str,
        auth: Option<String>,
        body: Value,
    ) -> Result<Reply, String> {
        self.client(auth).post(path, body).await
    }
}
