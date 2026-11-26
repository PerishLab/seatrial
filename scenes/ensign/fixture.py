import base64
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import subprocess
import threading
import time
from typing import override


def encode(value):
    return base64.urlsafe_b64encode(value).rstrip(b"=").decode()


class Fixture:
    def __init__(self, root):
        self.mode = "valid"
        self.count = 0
        self.key = Path(root) / "signing.pem"
        self.invoke(["openssl", "genpkey", "-algorithm", "EC", "-pkeyopt", "ec_paramgen_curve:P-256", "-out", str(self.key)])
        self.key.chmod(0o600)
        point = self.invoke(["openssl", "pkey", "-in", str(self.key), "-pubout", "-outform", "DER"])[-65:]
        assert point[0] == 4 and len(point) == 65
        self.jwk = {"kty": "EC", "crv": "P-256", "use": "sig", "alg": "ES256", "kid": "fixture", "x": encode(point[1:33]), "y": encode(point[33:])}
        self.server = ThreadingHTTPServer(("0.0.0.0", 8080), Handler)
        self.server.fixture = self
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def invoke(self, args, data=None):
        result = subprocess.run(args, input=data, capture_output=True, timeout=15)
        assert result.returncode == 0, "fixture OpenSSL operation failed"
        return result.stdout

    def token(self):
        self.count += 1
        now = int(time.time())
        claims = {"iss": "https://identity.example/api", "sub": "fixture-subject", "aud": "cloud", "iat": now - 10, "exp": now + 120, "kind": "access"}
        changes = {"issuer": {"iss": "https://foreign.example/api"}, "subject": {"sub": "foreign-subject"}, "audience": {"aud": "foreign"}, "multiple": {"aud": ["cloud", "foreign"]}, "expired": {"exp": now - 1}, "future": {"iat": now + 60}, "purpose": {"kind": "id"}}
        claims.update(changes.get(self.mode, {}))
        header = {"alg": "ES256", "typ": "JWT", "kid": "fixture"}
        body = encode(json.dumps(header).encode()) + "." + encode(json.dumps(claims).encode())
        signature = self.invoke(["openssl", "dgst", "-sha256", "-sign", str(self.key)], body.encode())
        assert signature[0] == 48 and signature[2] == 2
        size = signature[3]
        first = signature[4:4 + size].lstrip(b"\0").rjust(32, b"\0")
        offset = 4 + size
        assert signature[offset] == 2
        last = signature[offset + 2:offset + 2 + signature[offset + 1]].lstrip(b"\0").rjust(32, b"\0")
        signature = first + last
        if self.mode == "signature":
            signature = bytes([signature[0] ^ 1]) + signature[1:]
        return body + "." + encode(signature)

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()


class Handler(BaseHTTPRequestHandler):
    @override
    def log_message(self, *args):
        pass

    def reply(self, code, value):
        body = json.dumps(value).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    @override
    def do_GET(self):
        if hasattr(self.server,"relay"):
            return self.server.relay(self)
        issuer = "https://identity.example/api"
        match self.path:
            case "/api/whoami":
                self.reply(200, {"sub": "fixture-subject", "login": "fixture", "name": "fixture"})
            case "/api/.well-known/jwks.json":
                self.reply(200, {"keys": [self.server.fixture.jwk]})
            case "/api/.well-known/openid-configuration":
                self.reply(200, {"issuer": issuer, "token_endpoint": issuer + "/token", "jwks_uri": issuer + "/.well-known/jwks.json"})
            case _:
                self.reply(404, {})

    @override
    def do_POST(self):
        if hasattr(self.server,"relay"):
            return self.server.relay(self)
        length = int(self.headers.get("Content-Length", "0"))
        assert length < 65536
        self.rfile.read(length)
        match self.path:
            case "/api/bearer":
                self.reply(201, {"token": "INERT_SCENE_LOGIN_BEARER"})
            case "/api/token":
                self.reply(200, {"access_token": self.server.fixture.token(), "issued_token_type": "urn:ietf:params:oauth:token-type:access_token", "token_type": "Bearer", "expires_in": 120})
            case _:
                self.reply(404, {})

    @override
    def do_PATCH(self):
        self.server.relay(self)

    @override
    def do_DELETE(self):
        self.server.relay(self)
