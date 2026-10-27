use std::path::PathBuf;
use std::time::Instant;

use crate::docker::{must, words};
use crate::fetch::{self, Product};
use crate::lane::{Lane, Spec};
use crate::pin::{Pins, Plumb};
use crate::probe;

pub async fn run(pins: &Pins) -> Result<bool, String> {
    let product = pins.plumb.as_ref().ok_or("writer needs [plumb] pins")?;
    let lane = Lane::open(PathBuf::from(".local")).await?;
    println!("lane    {} (perish.owner=seatrial)", lane.ns);
    let start = Instant::now();
    let result = trial(&lane, pins, product).await;
    let ports = lane.ports().await;
    let cleanup = lane.close().await;
    let left = cleanup?;
    if !left.is_empty() {
        return Err(format!("lane {} left behind: {}", lane.ns, left.join(", ")));
    }
    println!(
        "lane    {} torn down; nothing labelled with it remains",
        lane.ns
    );
    let (_, published) = ports?;
    if published != 0 {
        return Err("writer published a host port".into());
    }
    match result {
        Ok(()) => {
            println!("pass    writer {:.3}s", start.elapsed().as_secs_f64());
            Ok(true)
        }
        Err(error) => {
            println!(
                "fail    writer {:.3}s {error}",
                start.elapsed().as_secs_f64()
            );
            Ok(false)
        }
    }
}

async fn trial(lane: &Lane, pins: &Pins, product: &Plumb) -> Result<(), String> {
    for image in [&pins.images.probe, &product.image] {
        must(&words(&["pull", "--quiet", image])).await?;
    }
    probe::start(lane, &pins.images.probe).await?;
    let binary = lane.dir.join("plumb");
    let release = fetch::binary(
        lane,
        Product {
            name: "plumb",
            channel: &product.channel,
            pointer: format!(
                "{}/v1/channels/{}.json",
                product.authority.trim_end_matches('/'),
                product.channel
            ),
        },
        &binary,
    )
    .await?;
    println!(
        "plumb   {} at {}, {} channel, archive sha256:{}",
        release.version, release.commit, product.channel, release.archive
    );
    let digest = must(&words(&[
        "image",
        "inspect",
        "--format",
        "{{index .RepoDigests 0}}",
        &product.image,
    ]))
    .await?;
    println!("image   {}", digest.text());
    let volume = lane.volume("writer").await?;
    let output = lane
        .once(&Spec {
            image: product.image.clone(),
            entry: Some("sh".into()),
            env: vec!["HOME=/trial/home".into(), "PLUMB_HOME=/trial/state".into()],
            mounts: vec![
                format!("{volume}:/trial"),
                format!("{}:/opt/plumb:ro", binary.display()),
            ],
            args: words(&["-eu", "-c", SCRIPT]),
            ..Spec::default()
        })
        .await?;
    if !output.ok {
        return Err(format!("writer: {} {}", output.err, output.text()));
    }
    println!("{}", output.text());
    Ok(())
}

const SCRIPT: &str = r#"
mkdir -p /trial/home /trial/source
git --version
/opt/plumb --version
git init -q --bare -b main /trial/origin.git
cd /trial/source
git init -q -b main
git config user.name Seatrial
git config user.email seatrial@example.invalid
printf '%s\n' retained > retained
git add retained
git commit -q -m base
git remote add origin git@github.com:Example/probe.git
git push -q /trial/origin.git main
cat > /trial/transport <<'TRANSPORT'
#!/bin/sh
exec git-upload-pack /trial/origin.git
TRANSPORT
chmod +x /trial/transport
git config core.sshCommand /trial/transport
git config ssh.variant simple
git fetch -q origin
git --git-dir=/trial/origin.git show-ref > /trial/refs.before
git show-ref > /trial/local.before
git status --porcelain --untracked-files=all > /trial/source.before
git worktree list --porcelain > /trial/trees.before
cat > /trial/gh <<'GH'
#!/bin/sh
printf '%s\n' "$*" >> /trial/calls
case "$*" in
  'api repos/Example/probe/issues?state=open&per_page=100 --paginate --slurp')
    printf '%s\n' '[[{"number":17,"type":{"name":"Auto"},"body":"<!-- plumb.auto/v1 operation=follow -->"}]]' ;;
  'api repos/Example/probe/issues/17')
    printf '%s\n' '{"user":{"id":1,"login":"fixture-writer[bot]"}}' ;;
  'api graphql -f query=query{viewer{databaseId login}}')
    printf '%s\n' '{"data":{"viewer":{"databaseId":2,"login":"foreign-operator"}}}' ;;
  *)
    printf '%s\n' "$*" > /trial/unexpected
    printf '%s\n' 'unexpected provider operation' >&2
    exit 97 ;;
esac
GH
chmod +x /trial/gh
status=0
/opt/plumb follow /trial/source --github-command /trial/gh --json > /trial/out 2> /trial/err || status=$?
cat /trial/err
test "$status" = 1
grep -F 'operation writer fixture-writer[bot] (id 1), but this follow runs as foreign-operator (id 2)' /trial/err
test ! -e /trial/unexpected
grep -Fx 'api repos/Example/probe/issues?state=open&per_page=100 --paginate --slurp' /trial/calls
grep -Fx 'api repos/Example/probe/issues/17' /trial/calls
grep -Fx 'api graphql -f query=query{viewer{databaseId login}}' /trial/calls
git --git-dir=/trial/origin.git show-ref > /trial/refs.after
git show-ref > /trial/local.after
git status --porcelain --untracked-files=all > /trial/source.after
git worktree list --porcelain > /trial/trees.after
cmp /trial/refs.before /trial/refs.after
cmp /trial/local.before /trial/local.after
cmp /trial/source.before /trial/source.after
cmp /trial/trees.before /trial/trees.after
test ! -e /trial/source/auto
printf '%s\n' 'writer refused; only allowed provider reads occurred; remote refs, source and worktrees unchanged'
"#;
