from client import Client
from fixture import Handler
import http.client
from http.server import ThreadingHTTPServer
import json
from pathlib import Path
import secrets
import sys
import threading
import time


class Relay:
    def __init__(self, host):
        self.host = host
        self.count = 0
        self.identity = None
        self.server = ThreadingHTTPServer(("0.0.0.0", 8080), Handler)
        self.server.relay = self.request
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def request(self, handler):
        length = int(handler.headers.get("Content-Length", "0"))
        assert length < 1048576
        body = handler.rfile.read(length)
        headers = {name: handler.headers[name] for name in ["authorization", "content-type", "cookie"] if name in handler.headers}
        connection = http.client.HTTPConnection(self.host, 8080, timeout=10)
        connection.request(handler.command, handler.path, body, headers)
        response = connection.getresponse()
        received = response.read(1048576)
        if handler.path == "/api/whoami" and response.status == 200:
            self.identity = json.loads(received)
        if handler.path == "/api/token":
            self.count += 1
        handler.send_response(response.status)
        handler.send_header("Content-Type", response.getheader("Content-Type", "application/json"))
        handler.send_header("Content-Length", str(len(received)))
        handler.end_headers()
        handler.wfile.write(received)
        connection.close()

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()


class Estate:
    def __init__(self, host, relay):
        self.host = host
        self.relay = relay
        self.sudo = Path("/sudo/token").read_text().strip()

    def call(self, method, path, value):
        body = None if value is None else json.dumps(value).encode()
        connection = http.client.HTTPConnection(self.host, 8080, timeout=10)
        connection.request(method, "/api" + path, body, {"content-type": "application/json", "authorization": "sudo " + self.sudo})
        response = connection.getresponse()
        data = response.read(1048576)
        connection.close()
        return response.status, json.loads(data) if data else None

    def ready(self):
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            try:
                status, body = self.call("GET", "/.well-known/openid-configuration", None)
                if status == 200:
                    assert body["issuer"] == "https://identity.example/api"
                    return
            except (OSError, http.client.HTTPException):
                pass
            time.sleep(0.2)
        raise AssertionError("released Ensign did not become ready")

    def join(self, name):
        status, card = self.call("POST", "/invite", {"note": name})
        assert status == 201
        password = secrets.token_urlsafe(24)
        status, actor = self.call("POST", "/join", {"code": card["code"], "login": name, "name": name, "pass": password})
        assert status == 201
        client = Client("/trial/" + name)
        client.login(name, password)
        client.invoke(["/opt/ensign", "whoami"])
        identity = self.relay.identity
        assert identity["id"] == actor["id"] and identity["login"] == name
        assert identity["sub"] and identity["sub"] != str(actor["id"])
        return client, actor["id"], identity["sub"]


def main():
    relay = Relay(sys.argv[1])
    try:
        estate = Estate(sys.argv[1], relay)
        estate.ready()
        client, actor, subject = estate.join("alice")
        state = client.acquire("genuine", {"subject": subject}, 0)
        assert client.status(state)["federation"]["tokenPresent"] is True
        client.call(state, ["auth", "begin"], 1)
        for name, values in [("issuer", {"issuer": "https://foreign.example/api"}), ("subject", {"subject": "foreign-subject"})]:
            state = client.acquire(name, dict({"subject": subject}, **values), 1)
            assert client.status(state)["pending"] is None
        state = client.acquire("audience", {"subject": subject, "audience": "unregistered"}, 1)
        assert client.status(state)["pending"]["retainedReturn"] is False
        count = relay.count
        client.call(state, ["auth", "recover"], 1)
        assert relay.count == count
        status, _ = estate.call("PATCH", "/Actor/" + str(actor), {"barred": "true"})
        assert status in [200, 204]
        state = client.acquire("barred", {"subject": subject}, 1)
        assert client.status(state)["federation"]["tokenPresent"] is False
        bob, _, subject = estate.join("bob")
        bob.invoke(["/opt/ensign", "logout"])
        state = bob.acquire("revoked", {"subject": subject}, 1)
        assert bob.status(state)["pending"] is None
        carol, actor, subject = estate.join("carol")
        status, _ = estate.call("DELETE", "/Actor/" + str(actor), None)
        assert status in [200, 204]
        state = carol.acquire("ended", {"subject": subject}, 1)
        assert carol.status(state)["federation"]["tokenPresent"] is False
        print(json.dumps({"case": "ensign-actual", "checks": ["genuine-login", "external-subject", "public-sdk-exchange", "explicit-loopback-source", "canonical-issuer", "wrong-subject", "unregistered-audience", "no-replay", "separate-oauth-custody", "barred", "revoked-login", "ended"], "sts": "not dispatched", "cloud": "not qualified"}))
    finally:
        relay.close()


if __name__ == "__main__":
    main()
