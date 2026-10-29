import hashlib
import json
import io
import tarfile
import tomllib
import os
import pathlib
import shutil
import subprocess

scene = pathlib.Path(os.environ["SEATRIAL_SCENE"])
work = pathlib.Path(os.environ["SEATRIAL_WORK"])
project = work / "conformance"
project.mkdir()
shutil.copy2(scene / "Cargo.toml.in", project / "Cargo.toml")
shutil.copytree(scene / "src", project / "src")
(project / ".cargo").mkdir()
(project / ".cargo/config.toml").write_text('[registries.perish]\nindex="sparse+https://cargo.perish.uk/"\n')
base = ["cargo"]
candidate = os.environ.get("SEATRIAL_RADICULA_CANDIDATE")
identity = {"kind": "formal-package"}
if candidate:
    root = pathlib.Path(candidate).resolve(strict=True)
    assert not subprocess.check_output(["git", "-C", str(root), "status", "--porcelain"])
    identity = {"kind": "candidate-source", "commit": subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip(), "tree": subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD^{tree}"], text=True).strip()}
    archive = subprocess.check_output(["git", "-C", str(root), "archive", identity["commit"]])
    identity["archiveSha256"] = hashlib.sha256(archive).hexdigest()
    snapshot = work / "candidate"
    snapshot.mkdir()
    with tarfile.open(fileobj=io.BytesIO(archive)) as material:
        material.extractall(snapshot, filter="data")
    base += ["--config", 'patch.perish.radicula-lock.path=' + json.dumps(str(snapshot / "crates/lock"))]
subprocess.run(base + ["generate-lockfile"], cwd=project, check=True, timeout=120)
metadata = subprocess.check_output(base + ["metadata", "--locked", "--format-version", "1"], cwd=project, timeout=120)
(work / "conformance-metadata.json").write_bytes(metadata)
packages = json.loads(metadata)["packages"]
root = next(package for package in packages if package["name"] == "radicula-lock")
if not candidate:
    assert root["source"] == "registry+sparse+https://cargo.perish.uk/"
    def raw(url):
        return subprocess.check_output(["curl", "--fail", "--silent", "--show-error", "--retry", "2", "--max-time", "30", url])
    def fetch(url):
        return json.loads(raw(url))
    authority = "https://releases.radicula.perish.uk"
    pointer = fetch(authority + "/v1/channels/stable.json")
    seal = raw(pointer["seal"]["url"])
    assert len(seal) == pointer["seal"]["size"]
    assert hashlib.sha256(seal).hexdigest() == pointer["seal"]["sha256"]
    seal = json.loads(seal)
    assert seal["commit"] == pointer["commit"]
    marker = "v" + root["version"]
    assert pointer["releaseVersion"].lstrip("v") == root["version"]
    distribution = fetch(authority + "/v1/releases/stable/" + marker + "/distribution.json")
    assert distribution["state"] == "complete"
    vcs = json.loads((pathlib.Path(root["manifest_path"]).parent / ".cargo_vcs_info.json").read_text())
    assert vcs["git"]["sha1"] == distribution["commit"]
    identity.update({"package": root["name"], "version": root["version"], "source": distribution["commit"], "pointer": pointer, "distribution": distribution, "seal": seal})
subprocess.run(base + ["build", "--locked", "--target-dir", str(work / "target")], cwd=project, check=True, timeout=120)
binary = work / "target/debug/seatrial-radicula"
identity.update({"binarySha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "lockSha256": hashlib.sha256((project / "Cargo.lock").read_bytes()).hexdigest()})
identity["resolution"] = tomllib.loads((project / "Cargo.lock").read_text())["package"]
if not candidate:
    assert fetch(authority + "/v1/channels/stable.json") == pointer
(work / "conformance-identity.json").write_text(json.dumps(identity, indent=2))
print(json.dumps(identity))
