import json
from pathlib import Path
import subprocess
import time


ROOT = Path("/trial")
STATE = ROOT / "provider.json"
ENV = {"HOME": str(ROOT / "home"), "PATH": "/usr/local/bin:/usr/bin:/bin"}


def initial():
    project = {"__typename": "ProjectV2", "id": "PVT_trial", "number": 1, "title": "Planning", "url": "https://github.com/orgs/Example/projects/1", "updatedAt": "2026-10-21T00:00:00Z", "closed": False, "public": False, "readme": "Trial boundary", "shortDescription": "Recovery", "viewerCanUpdate": True, "owner": {"__typename": "Organization", "id": "O_trial", "login": "Example"}}
    issue = {"__typename": "Issue", "id": "I_trial", "number": 4, "url": "https://github.com/Example/probe/issues/4", "state": "OPEN", "updatedAt": "issue-original", "issueType": {"id": "IT_task", "name": "Task"}, "repository": {"id": "R_trial", "owner": {"id": "O_trial"}}}
    return {"project": project, "issue": issue, "fields": [], "items": [], "views": [], "workflows": [], "repositories": [], "statusUpdates": [], "writes": [], "hidden": False, "observations": 0, "calls": 0}


def state():
    return json.loads(STATE.read_text())


def invoke(args):
    before = state()["calls"]
    started = time.monotonic()
    result = subprocess.run(["/opt/concord", "--config", str(ROOT / "concord.toml"), "--home", str(ROOT / "data"), "--json", "project", *args, "--github-command", str(ROOT / "provider")], env=ENV, capture_output=True, text=True, timeout=60)
    print(json.dumps({"command": args[0], "seconds": time.monotonic() - started, "exit": result.returncode, "provider_calls": state()["calls"] - before}), flush=True)
    return result


def decode(result):
    assert result.returncode == 0, result.stderr
    return json.loads(result.stdout)


def main():
    ROOT.joinpath("home").mkdir()
    ROOT.joinpath("concord.toml").write_text("home = " + json.dumps(str(ROOT / "data")) + "\n")
    subprocess.run(["install", "-m", "755", "/scene/project.sh", str(ROOT / "provider")], check=True)
    STATE.write_text(json.dumps(initial()))
    intent = ROOT / "intent.json"
    intent.write_text(json.dumps({"reason": "Verify recovery without replay in an isolated trial.", "action": {"kind": "candidates", "issues": ["I_trial"], "present": True}}))
    plan = decode(invoke(["prepare", "Example/1", "--input", str(intent)]))
    path = ROOT / "plan.json"
    path.write_text(json.dumps(plan))
    refused = invoke(["apply", "--apply", "--input", str(path)])
    assert refused.returncode != 0 and refused.stdout == "", "unconfirmed write reported success"
    error = json.loads(refused.stderr)["error"]
    assert error["code"] == "concord.project.indeterminate", error
    assert error["details"]["unconfirmed_step"] == 0, error
    observed = state()
    assert observed["observations"] == 3, observed
    assert observed["writes"].count("addProjectV2ItemById") == 1, observed
    assert observed["writes"].count("createProjectV2StatusUpdate") == 1, observed
    observed["hidden"] = False
    STATE.write_text(json.dumps(observed))
    recovered = decode(invoke(["recover", "--input", str(path)]))
    assert recovered["project"]["desired_state"] == "observed", recovered
    assert state()["writes"] == observed["writes"], "recover replayed a write"
    assert not ROOT.joinpath("data/workspace").exists(), "remote-only Project command created an estate"
    print(json.dumps({"result": "observed-without-replay", "error": error["code"], "unconfirmed_step": error["details"]["unconfirmed_step"], "desired_state": recovered["project"]["desired_state"], "writes": state()["writes"], "hidden_observations": observed["observations"], "provider_calls": state()["calls"]}), flush=True)


if __name__ == "__main__":
    main()
