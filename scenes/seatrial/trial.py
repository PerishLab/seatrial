import json
import hashlib
import os
from pathlib import Path
import re
import signal
import subprocess


def placement():
    return {"artifact": "linux-x64-deb", "entry": "locus-api", "paths": {"locus-api": "usr/bin/locus-api"}}


def configuration(path, frame, name):
    path.mkdir(mode=0o700)
    chosen = placement()
    if name == "negative":
        chosen["paths"]["locus-api"] = "../usr/bin/locus-api"
    text = '[images]\nprobe=' + json.dumps(frame.probe) + '\n[locus]\nchannel="stable"\nauthority="https://releases.locus.perish.uk"\n'
    text += '[locus.placement]\nartifact="linux-x64-deb"\nentry="locus-api"\npaths={"locus-api"=' + json.dumps(chosen["paths"]["locus-api"]) + '}\n'
    (path / "pins.toml").write_text(text)


def inventory(lane, kind):
    assert re.fullmatch(r"st[0-9a-f]{8}", lane)
    selectors = ["--filter", "label=perish.owner=seatrial", "--filter", "label=perish.lane=" + lane]
    found = subprocess.run(["docker", kind, "ls", "-q", *selectors], capture_output=True, text=True, check=True)
    return found.stdout.split()


def cleanup(lane):
    for kind in ["container", "volume", "network"]:
        assert not inventory(lane, kind), (lane, kind)


def reclaim(lane):
    commands = {"container": ["rm", "-f"], "volume": ["rm", "-f"], "network": ["rm"]}
    for kind, command in commands.items():
        ids = inventory(lane, kind)
        if ids:
            subprocess.run(["docker", kind, *command, *ids], capture_output=True, check=True, timeout=30)
    cleanup(lane)


def invoke(args, path, env):
    process = subprocess.Popen(args, cwd=path, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    expired = False
    try:
        output, error = process.communicate(timeout=120)
    except subprocess.TimeoutExpired:
        expired = True
        process.send_signal(signal.SIGTERM)
        try:
            output, error = process.communicate(timeout=15)
        except subprocess.TimeoutExpired:
            process.kill()
            output, error = process.communicate(timeout=5)
    return {"exit": process.returncode, "stdout": output, "stderr": error, "expired": expired}


def run(root, frame, name):
    path = root / name
    configuration(path, frame, name)
    report = root / (name + "-report.json")
    home = path / "home"
    home.mkdir(mode=0o700)
    env = {"PATH": os.defpath, "HOME": str(home), "LANG": "C.UTF-8"}
    args = [frame.binary, "run", "--pins", str(path / "pins.toml"), "--scenes", str(frame.fixture), "--case", "placement-fixture", "--report", str(report)]
    result = invoke(args, path, env)
    lanes = set(re.findall(r"^lane\s+(st[0-9a-f]{8})\b", result["stdout"], re.MULTILINE))
    assert len(lanes) == 1, result
    lane = next(iter(lanes))
    try:
        assert not result["expired"], result
        record = json.loads(report.read_text())
        assert record["lane"] == lane
        assert record["runner"] == hashlib.sha256(Path(frame.binary).read_bytes()).hexdigest()
        assert record["scene"] is None and record["cleanup"], record
        cleanup(lane)
        actual = None
        for line in result["stdout"].splitlines():
            if line.startswith("PLACEMENT_ACTUAL="):
                assert actual is None
                actual = json.loads(line.removeprefix("PLACEMENT_ACTUAL="))
        return {**result, "report": record, "actual": actual}
    finally:
        reclaim(lane)
