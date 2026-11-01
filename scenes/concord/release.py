import json

import context
from proxy import Proxy


def main():
    context.setup()
    (context.ROOT / "fixture.json").write_text(json.dumps({"mode": "release"}))
    for failures, expected in [(set(), "exact distribution record is absent"), ({1}, "first distribution read failed"), ({2}, "second distribution read failed"), ({1, 2}, "both distribution reads failed")]:
        with Proxy(failures) as proxy:
            result = context.concord(["acceptance", "evaluate", "Example/probe#83", "--github-command", str(context.ROOT / "provider")], {"HTTPS_PROXY": proxy.address(), "https_proxy": proxy.address()})
            assert proxy.server.count == 2, "distribution did not perform exactly two reads"
        evaluation = result["acceptance"]["evaluation"]
        assert evaluation["verdict"] == "unknown"
        assert "SCENE_SECRET" not in json.dumps(result)
        checks = {item["name"]: item["fact"] for item in evaluation["checks"]}
        assert checks["release"]["verdict"] == "unknown"
        assert expected in checks["release"]["reason"], checks["release"]
        assert all(value["verdict"] == "satisfied" for name, value in checks.items() if name != "release"), checks
        print(f"release observation: {expected}; exactly two reads, unknown retained")
    assert not (context.ROOT / "space" / ".concord").exists()


if __name__ == "__main__":
    main()
