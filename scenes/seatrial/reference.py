import hashlib
import json
from pathlib import Path
import subprocess

AUTHORITY = "https://releases.locus.perish.uk"
SCRIPT = r'''
import hashlib, json, os, subprocess, tarfile
process = subprocess.Popen(["dpkg-deb", "--fsys-tarfile", "/fixture.deb"], stdout=subprocess.PIPE)
selected = []
with tarfile.open(fileobj=process.stdout, mode="r|") as archive:
    for entry in archive:
        if entry.name.removeprefix("./") == "usr/bin/locus-api":
            assert entry.isfile() and entry.mode & 0o111 and entry.size <= 33554432
            data = archive.extractfile(entry).read(33554433)
            assert len(data) == entry.size
            selected.append(data)
assert process.wait(timeout=10) == 0 and len(selected) == 1
path = "/tmp/reference-locus-api"
with open(path, "xb") as output: output.write(selected[0])
os.chmod(path, 0o700)
version = subprocess.run([path, "--version"], capture_output=True, text=True, check=True).stdout.strip()
print(json.dumps({"sha256": hashlib.sha256(selected[0]).hexdigest(), "version": version}))
'''


def fetch(url, limit, frame):
    assert url.startswith(AUTHORITY + "/"), url
    args = ["docker", "run", "--rm", "--name", frame.lane + "-placement-fetch", "--network", frame.lane,
            "--label", "perish.owner=seatrial", "--label", "perish.lane=" + frame.lane,
            "--memory", "128m", "--pids-limit", "32", "--entrypoint", "curl", frame.probe,
            "-q", "-fLsS", "--proto", "=https", "--proto-redir", "=https", "--max-time", "60",
            "--max-filesize", str(limit), url]
    result = subprocess.run(args, capture_output=True, timeout=75)
    assert result.returncode == 0, result.stderr
    assert len(result.stdout) <= limit
    return result.stdout


def objected(reference, limit, frame):
    data = fetch(reference["url"], limit, frame)
    assert len(data) == reference["size"]
    assert hashlib.sha256(data).hexdigest() == reference["sha256"]
    return data


def resolve(root, frame):
    pointer = json.loads(fetch(AUTHORITY + "/v1/channels/stable.json", 4194304, frame))
    assert pointer["product"] == "locus" and pointer["channel"] == "stable"
    seal = json.loads(objected(pointer["seal"], 4194304, frame))
    for key in ["product", "releaseVersion", "commit", "channel"]:
        assert pointer[key] == seal[key]
    artifact = seal["artifacts"]["linux-x64-deb"]
    data = objected(artifact, 67108864, frame)
    path = root / "fixture.deb"
    path.write_bytes(data)
    args = ["docker", "run", "--rm", "--name", frame.lane + "-placement-reference", "--network", frame.lane,
            "--label", "perish.owner=seatrial", "--label", "perish.lane=" + frame.lane,
            "--memory", "256m", "--pids-limit", "64",
            "--mount", "type=bind,src=" + str(path) + ",dst=/fixture.deb,readonly",
            "--entrypoint", "python3", frame.image, "-c", SCRIPT]
    result = subprocess.run(args, capture_output=True, text=True, timeout=90)
    assert result.returncode == 0, result.stderr
    actual = json.loads(result.stdout)
    assert actual["version"] == "locus-api " + seal["releaseVersion"]
    return {"name": "locus", "channel": "stable", "version": seal["releaseVersion"], "commit": seal["commit"],
            "seal": pointer["seal"]["sha256"], "archive": artifact["sha256"], "file": actual}
