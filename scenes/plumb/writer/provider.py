from http.client import parse_headers
import json
from pathlib import Path
import re
from socketserver import BaseRequestHandler, TCPServer
import ssl
import subprocess
from threading import Thread


TRIAL = Path("/trial")
TRUST = TRIAL / "trust"
TOKEN = "seatrial-writer-fixture"
ISSUES = "/repos/Example/probe/issues"
INVENTORY = f"{ISSUES}?state=open&per_page=30&page=1"
TICKET = f"{ISSUES}/17"
VIEWER = "/graphql"
ISSUE = {
    "node_id": "I_writer_fixture",
    "number": 17,
    "html_url": "https://github.com/Example/probe/issues/17",
    "repository_url": "https://api.github.com/repos/Example/probe",
    "title": "Writer fixture",
    "body": "<!-- plumb.auto/v1 operation=follow -->",
    "updated_at": "2026-01-01T00:00:00Z",
    "state": "open",
    "state_reason": None,
    "type": {"name": "Auto"},
    "user": {"id": 1, "login": "fixture-writer[bot]"},
    "labels": [],
}
ACTOR = {"data": {"viewer": {
    "id": "U_foreign_fixture", "databaseId": 2, "login": "foreign-operator",
}}}


def request(stream):
    line = stream.readline(4096).decode("ascii")
    method, route, version = line.rstrip("\r\n").split(" ")
    if version != "HTTP/1.1":
        raise ValueError("unexpected HTTP version")
    return method, route, parse_headers(stream)


def response(method, route, headers, stream):
    if headers.get("Authorization") != f"Bearer {TOKEN}" or headers.get("Transfer-Encoding"):
        return None
    length = headers.get("Content-Length", "0")
    if not length.isdecimal() or int(length) > 4096:
        return None
    if method == "GET" and int(length) == 0:
        return {INVENTORY: [ISSUE], TICKET: ISSUE}.get(route)
    if method == "POST" and route == VIEWER and int(length) > 0:
        data = json.loads(stream.read(int(length)))
        query = data.get("query", "")
        if re.fullmatch(r"query RunsealViewer\s*\{\s*viewer\s*\{\s*id\s+databaseId\s+login\s*\}\s*\}", query):
            return ACTOR
    return None


class Tunnel(BaseRequestHandler):
    def handle(self):
        self.request.settimeout(5)
        with self.request.makefile("rb") as stream:
            method, route, _ = request(stream)
        if (method, route) != ("CONNECT", "api.github.com:443"):
            self.server.unexpected.append((method, route))
            self.request.sendall(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
            return
        self.request.sendall(b"HTTP/1.1 200 Connection established\r\n\r\n")
        with self.server.tls.wrap_socket(self.request, server_side=True) as secure:
            self.serve(secure)

    def serve(self, secure):
        with secure.makefile("rb") as stream:
            method, route, headers = request(stream)
            self.server.calls.append((method, route))
            payload = response(method, route, headers, stream)
        if payload is None:
            self.server.unexpected.append((method, route))
            secure.sendall(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            return
        body = json.dumps(payload).encode()
        headers = (
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n"
            f"Content-Length: {len(body)}\r\nConnection: close\r\n\r\n"
        )
        secure.sendall(headers.encode("ascii") + body)


def main():
    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    tls.load_cert_chain(TRUST / "server.pem", TRUST / "server.key")
    with TCPServer(("127.0.0.1", 0), Tunnel) as server:
        server.tls = tls
        server.calls = []
        server.unexpected = []
        thread = Thread(target=server.serve_forever)
        thread.start()
        try:
            proxy = f"http://127.0.0.1:{server.server_address[1]}"
            with (TRIAL / "out").open("w") as out, (TRIAL / "err").open("w") as err:
                result = subprocess.run([
                    "env", f"HTTPS_PROXY={proxy}", f"https_proxy={proxy}",
                    "NO_PROXY=depot.plumb.perish.uk", "no_proxy=depot.plumb.perish.uk",
                    "/opt/plumb", "follow", str(TRIAL / "source"),
                    "--root-ca-file", str(TRUST / "ca.pem"), "--token-file", str(TRUST / "token"),
                    "--json",
                ], stdout=out, stderr=err, timeout=90)
        finally:
            server.shutdown()
            thread.join()
        reads = {("GET", INVENTORY), ("GET", TICKET), ("POST", VIEWER)}
        if result.returncode != 1 or server.unexpected or set(server.calls) != reads:
            raise RuntimeError(
                f"follow exited {result.returncode}, calls={server.calls}, "
                f"unexpected={server.unexpected}\n{(TRIAL / 'err').read_text()}"
            )


if __name__ == "__main__":
    main()
