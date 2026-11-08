import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else "/trial").resolve()
BINARY = sys.argv[2] if len(sys.argv) > 2 else "/opt/concord"
SOURCE = Path(sys.argv[3] if len(sys.argv) > 3 else "/scene").resolve()
VALUES = {"HOME": str(ROOT / "home"), "PATH": "/usr/local/bin:/usr/bin:/bin"}


def invoke(args, extra=None):
    result = subprocess.run(args, env=dict(VALUES, **(extra or {})), capture_output=True, text=True, timeout=60)
    assert result.returncode == 0, f"command refused: {result.stderr}"
    return result.stdout


def setup():
    ROOT.mkdir(parents=True, exist_ok=True)
    for name in ["home", "data"]:
        (ROOT / name).mkdir()
    (ROOT / "concord.toml").write_text("home = " + json.dumps(str(ROOT / "data")) + "\n")
    assert not (ROOT / "fixture.json").exists()
    invoke(["install", "-m", "755", str(SOURCE / "provider.sh"), str(ROOT / "provider")])
    VALUES.update({"SEATRIAL_CONCORD_SOURCE": str(SOURCE), "SEATRIAL_CONCORD_FIXTURE": str(ROOT / "fixture.json")})
    invoke([BINARY, "--version"])


def concord(args, extra=None):
    return json.loads(invoke([BINARY, "--config", str(ROOT / "concord.toml"), "--home", str(ROOT / "data"), "--json", *args], extra))


def git(root, args):
    return invoke(["git", "-C", str(root), *args])


def main():
    setup()
    (ROOT / "fixture.json").write_text(json.dumps({"mode": "context"}))
    concord(["issue", "bootstrap"])
    workspace = ROOT / "data" / "workspace"
    identity = json.loads((workspace / "identity.json").read_text())
    assert identity["schema"] == "concord.space/v1"
    assert len(identity["id"]) == 64 and all(value in "0123456789abcdef" for value in identity["id"])
    for name in ["estate.sqlite3", "sudo"]:
        assert (workspace / name).is_file()
    source = ROOT / "source"
    source.mkdir()
    remote = ROOT / "remote.git"
    invoke(["git", "init", "--bare", "-b", "main", str(remote)])
    git(source, ["init", "-b", "main"])
    git(source, ["config", "user.name", "Seatrial"])
    git(source, ["config", "user.email", "seatrial@example.invalid"])
    (source / "retained").write_text("retained\n")
    git(source, ["add", "retained"])
    git(source, ["commit", "-m", "fixture"])
    git(source, ["remote", "add", "origin", "https://github.com/Example/probe.git"])
    git(source, ["config", f"url.{remote}.insteadOf", "https://github.com/Example/probe.git"])
    git(source, ["push", "-u", "origin", "main"])
    before = git(source, ["show-ref"])
    extra = {
        "GIT_DIR": str(ROOT / "missing.git"),
        "GIT_WORK_TREE": str(ROOT / "foreign"),
        "GIT_INDEX_FILE": str(ROOT / "foreign-index"),
        "GIT_NAMESPACE": "foreign",
        "GIT_CONFIG_COUNT": "1",
        "GIT_CONFIG_KEY_0": "remote.origin.url",
        "GIT_CONFIG_VALUE_0": "https://github.com/foreign/repository.git",
    }
    result = concord(["integration", "register", "Example/probe", "--path", str(source), "--github-command", str(ROOT / "provider")], extra)
    assert result["integration"]["repository"] == {"owner": "Example", "name": "probe"}
    assert before == git(source, ["show-ref"])
    assert git(source, ["status", "--porcelain"]) == ""
    assert not (ROOT / "foreign-index").exists()
    audit = concord(["audit"])
    assert audit["space"] == identity["id"] and audit["agreement"]["faults"] == []
    assert json.loads((workspace / "identity.json").read_text()) == identity
    assert (workspace / "locks" / "estate.lock").is_file()
    for path in [ROOT, source]:
        for name in [".concord", ".concord.lock", ".issues"]:
            assert not (path / name).exists()
    print("registered the subject under foreign Git context; refs and source retained")


if __name__ == "__main__":
    main()
