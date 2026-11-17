from client import Client
from fixture import Fixture
import json


def main():
    client = Client("/trial/claims")
    server = Fixture(client.root)
    try:
        client.login("fixture", "inert-fixture-password")
        state = client.acquire("valid", {}, 0)
        status = client.status(state)
        assert status["federation"]["tokenPresent"] is True
        assert status["pending"] is None
        completed = ["valid"]
        for mode in ["issuer", "subject", "audience", "multiple", "expired", "future", "purpose", "signature"]:
            server.mode = mode
            state = client.acquire(mode, {}, 1)
            status = client.status(state)
            assert status["federation"]["tokenPresent"] is False
            assert status["pending"]["retainedReturn"] is True
            count = server.count
            client.call(state, ["auth", "recover"], 1)
            assert server.count == count
            client.call(state, ["auth", "renew"], 1)
            assert server.count == count
            completed.append(mode)
        path = client.home / "state" / "pat.json"
        assert path.stat().st_mode & 0o777 == 0o600
        path.chmod(0o644)
        state = client.acquire("custody", {}, 1)
        assert client.status(state)["pending"] is None
        path.chmod(0o600)
        print(json.dumps({"case": "ensign-claims", "checks": completed + ["private-custody", "offline-recovery-no-replay"], "fixture": "independent local ES256 issuer", "sts": "not dispatched", "cloud": "not qualified"}))
    finally:
        server.close()


if __name__ == "__main__":
    main()
