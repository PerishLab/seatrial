import hashlib
import json
import pathlib
import subprocess
import sys

PLUMB = pathlib.Path("/opt/tools/plumb")
ROOT = pathlib.Path("/trial/source/probe")
EVIDENCE = pathlib.Path("/trial/evidence")
STATE = pathlib.Path(sys.argv[2])
HOME = pathlib.Path(sys.argv[3])
EXPECTED = sys.argv[1]
LIMIT = 96 * 1024
TOTAL = 128 * 1024


def call(*argv, cwd=ROOT, timeout=120):
    return subprocess.run(argv, cwd=cwd, capture_output=True, text=True, timeout=timeout)


def success(value):
    assert value.returncode == 0, value.stdout[-2000:] + value.stderr[-6000:]
    return json.loads(value.stdout)


def git(*argv):
    value = call("git", *argv)
    assert value.returncode == 0, value.stderr
    return value.stdout.strip()


def bounded(name, text):
    size = len(text.encode())
    assert size <= LIMIT, f"{name} exceeds {LIMIT} bytes: {size}"
    return text


def product():
    value = subprocess.check_output([PLUMB, "--version"], text=True).strip()
    assert value == "plumb " + EXPECTED, (value, EXPECTED)
    return {"version": value, "sha256": hashlib.sha256(PLUMB.read_bytes()).hexdigest()}


def fixture():
    assert HOME == pathlib.Path("/trial/home"), HOME
    assert pathlib.Path.home() == HOME, pathlib.Path.home()
    assert STATE == pathlib.Path("/trial/state"), STATE
    assert not STATE.exists(), "the lane must provide a fresh PLUMB_HOME"
    ROOT.mkdir(parents=True)
    EVIDENCE.mkdir(parents=True)
    git("init", "-q", "-b", "topic")
    git("config", "user.name", "Seatrial")
    git("config", "user.email", "seatrial@example.invalid")
    git("remote", "add", "origin", "https://github.com/Example/probe.git")
    (ROOT / "src").mkdir()
    (ROOT / "Cargo.toml").write_text(
        '[package]\nname="probe"\nversion="0.0.0"\nedition="2024"\n'
    )
    (ROOT / "Cargo.lock").write_text(
        'version=4\n[[package]]\nname="probe"\nversion="0.0.0"\n'
    )
    (ROOT / "src/lib.rs").write_text(
        "pub fn answer() -> u8 {\n    42\n}\n"
    )
    (ROOT / "plumb.toml").write_text(
        '[identity]\nns="seatrial-cache"\n'
        '[workflow.hash.guard]\nrust=["Cargo.toml","Cargo.lock","src"]\n'
    )
    git("add", ".")
    git("commit", "-q", "-m", "cache lifecycle fixture")
    assert git("status", "--porcelain", "--untracked-files=all") == ""


def cache():
    status = success(call(str(PLUMB), "cache", "status", "--json"))
    assert status["schema"] == "plumb.guard-cache/v1", status
    assert len(status["buckets"]) == 1, status
    entry = status["buckets"][0]
    assert entry["identity"] == str(ROOT / ".git"), entry
    assert entry["status"] == "live", entry
    assert entry["bytes"] > 0, entry
    assert entry["working"] > 0, entry
    return status, entry


def observation(products, guard, proof, data):
    diagnostic = data["diagnostic"]
    status = data["status"]
    entry = data["entry"]
    events = [json.loads(line) for line in diagnostic.splitlines()]
    finish = next(event for event in events if event["kind"] == "finish")
    raw = {
        "guard_stdout": bounded("guard stdout", guard.stdout),
        "guard_stderr": bounded("guard stderr", guard.stderr),
        "guard_diagnostic": bounded("guard diagnostic", diagnostic),
    }
    size = sum(len(value.encode()) for value in raw.values())
    assert size <= TOTAL, f"raw diagnostics exceed {TOTAL} bytes: {size}"
    return {
        "kind": "formal-tool conformance observation",
        "schema": "seatrial.plumb-cache-lifecycle/v1",
        "product": products,
        "fixture": {
            "repository": "Example/probe",
            "git_common_dir": str(ROOT / ".git"),
            "tree": git("rev-parse", "HEAD^{tree}"),
            "head": git("rev-parse", "HEAD"),
            "clean_after_guard": git("status", "--porcelain", "--untracked-files=all") == "",
        },
        "home": {
            "scope": "lane-private volume",
            "path": str(STATE),
            "outside_repository": not str(STATE).startswith(str(ROOT) + "/"),
        },
        "guard": {
            "exit": 0,
            "schema": proof.get("schema"),
            "digest": proof.get("digest"),
            "actions": proof.get("actions", []),
            "diagnostic_finish": finish.get("data", {}),
            "diagnostic_kinds": [event["kind"] for event in events],
        },
        "cache": {
            "schema": status["schema"],
            "bucket": entry["bucket"],
            "identity": entry["identity"],
            "status": entry["status"],
            "used": entry["used"],
            "bytes": entry["bytes"],
            "working": entry["working"],
        },
        "transport": {
            "declared": False,
            "invoked": False,
            "note": "No transport command was invoked; this case observes only the local Guard build cache.",
        },
        "raw": raw,
        "cleanup": "owned by the Seatrial runner after this observation is emitted",
    }


def main():
    products = product()
    fixture()
    path = EVIDENCE / "guard.jsonl"
    guard = call(str(PLUMB), "guard", ".", "--json", "--diagnostic", str(path))
    assert guard.returncode == 0, guard.stdout[-2000:] + guard.stderr[-6000:]
    proof = json.loads(guard.stdout)
    assert proof["schema"] == "plumb.guard-proof/v1", proof
    diagnostic = path.read_text()
    events = [json.loads(line) for line in diagnostic.splitlines()]
    finish = next(event for event in events if event["kind"] == "finish")
    assert finish["data"]["exit_code"] == 0, finish
    status, entry = cache()
    data = {"diagnostic": diagnostic, "status": status, "entry": entry}
    print(json.dumps(observation(products, guard, proof, data), sort_keys=True), flush=True)


if __name__ == "__main__":
    main()
