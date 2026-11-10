import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import time
from types import SimpleNamespace


def run(args):
    result = subprocess.run(["docker", *args], capture_output=True, text=True, timeout=20)
    assert result.returncode == 0, result.stderr
    return result.stdout.strip()


def inspect(identifier, lane):
    value = json.loads(run(["inspect", "--format", "{{json .}}", identifier]))
    labels = value["Config"]["Labels"]
    assert labels["perish.owner"] == "seatrial" and labels["perish.lane"] == lane
    assert not value["HostConfig"]["PortBindings"] and not value["HostConfig"]["PublishAllPorts"]
    return value


def ready(identifier, lane):
    deadline = time.monotonic() + 15
    address = "http://locus." + lane + ".internal:8080/health"
    while time.monotonic() < deadline:
        result = subprocess.run(["docker", "exec", identifier, "curl", "-q", "-fsS", "--noproxy", "*", "--max-time", "1", address],
                                capture_output=True, text=True, timeout=3)
        if result.returncode == 0:
            value = json.loads(result.stdout)
            assert value == {"status": "ok"}
            return value
        assert inspect(identifier, lane)["State"]["Running"]
        time.sleep(0.1)
    raise RuntimeError("owned Locus service health budget exceeded")


def service(frame):
    assert re.fullmatch(r"st[0-9a-f]+", frame.lane)
    assert frame.volume.startswith(frame.lane + "-")
    assert frame.binary.is_absolute() and frame.binary.is_file() and "," not in str(frame.binary)
    name = frame.lane + "-locus-service"
    args = ["run", "--detach", "--name", name, "--network", frame.lane,
            "--label", "perish.owner=seatrial", "--label", "perish.lane=" + frame.lane,
            "--memory", "256m", "--cpus", "1", "--pids-limit", "128",
            "--add-host", "locus." + frame.lane + ".internal:127.0.0.1",
            "--mount", "type=bind,src=" + str(frame.binary) + ",dst=/usr/local/bin/locus-api,readonly",
            "--mount", "type=volume,src=" + frame.volume + ",dst=/state",
            "--entrypoint", "/usr/local/bin/locus-api", frame.image,
            "serve", "--home", "/state", "--listen", "127.0.0.1:8080"]
    identifier = run(args)
    assert re.fullmatch(r"[0-9a-f]{64}", identifier)
    try:
        state = inspect(identifier, frame.lane)
        assert state["Config"]["Image"] == frame.image
        health = ready(identifier, frame.lane)
        version = run(["exec", identifier, "/usr/local/bin/locus-api", "--version"])
        assert version == "locus-api " + frame.version
        run(["kill", "--signal", "SIGINT", identifier])
        outcome = int(run(["wait", identifier]))
        assert outcome == 0
        print(json.dumps({"diagnostic": True, "qualification": False, "version": version,
                          "binary": hashlib.sha256(frame.binary.read_bytes()).hexdigest(),
                          "health": health, "exit": outcome, "image": frame.image}), flush=True)
    finally:
        inspect(identifier, frame.lane)
        run(["rm", "--force", identifier])
        assert not run(["ps", "--all", "--quiet", "--no-trunc", "--filter", "id=" + identifier])


def main():
    frame = SimpleNamespace(binary=Path(sys.argv[1]), lane=sys.argv[2], image=sys.argv[3],
                            volume=sys.argv[4], version=sys.argv[5])
    service(frame)


if __name__ == "__main__":
    main()
