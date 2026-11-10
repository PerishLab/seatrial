import json
from pathlib import Path
import signal
import subprocess
import time
import urllib.parse
import urllib.request


class Service:
    def __init__(self, root, product, hostname):
        self.root = Path(root)
        self.root.mkdir(mode=0o700)
        self.binary = str(product.binary)
        self.endpoint = "http://" + hostname + ":8080"
        self.env = {"HOME": str(self.root), "PATH": "/usr/bin:/bin", "LANG": "C.UTF-8"}
        self.reader = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        self.process = None
        self.log = (self.root / "service.log").open("xb")
        actual = subprocess.run([self.binary, "--version"], env=self.env, capture_output=True, text=True, check=True, timeout=10).stdout.strip()
        assert actual == "locus-api " + product.version, actual
        (self.root / "locus-api.toml").write_text("[retention]\ndays=1\n")
        self.start()

    def start(self):
        assert self.process is None or self.process.poll() is not None
        self.process = subprocess.Popen([self.binary, "serve", "--home", str(self.root), "--listen", "127.0.0.1:8080", "--interval", "100"], env=self.env, stdout=self.log, stderr=self.log)
        self.until(lambda: self.request("/health")[0] == b'{"status":"ok"}')
        assert self.process.poll() is None

    def request(self, path):
        with self.reader.open(self.endpoint + path, timeout=3) as response:
            assert response.status == 200
            body = response.read(4 * 1024 * 1024 + 1)
            assert len(body) <= 4 * 1024 * 1024
            return body, response.headers

    def query(self, **values):
        body, headers = self.request("/api/v1/atoms?" + urllib.parse.urlencode({"producer": "bagua", **values}))
        atoms = [json.loads(line) for line in body.splitlines()]
        assert all(atom.get("producer") == "bagua" and isinstance(atom.get("context"), dict) and isinstance(atom.get("at"), int) for atom in atoms)
        return atoms, headers

    def until(self, observe):
        end = time.monotonic() + 25
        last = None
        while time.monotonic() < end:
            try:
                result = observe()
                if result:
                    return result
            except (OSError, ValueError) as failure:
                last = type(failure).__name__
            time.sleep(0.05)
        raise AssertionError("Locus observation deadline exceeded: " + str(last))

    def events(self):
        return self.query()[0]

    def stop(self):
        if self.process is not None and self.process.poll() is None:
            self.process.send_signal(signal.SIGINT)
            assert self.process.wait(timeout=10) == 0

    def close(self):
        self.stop()
        self.log.close()


def payloads(atoms, event):
    return [atom for atom in atoms if atom.get("payload", {}).get("event") == event]


def trace(atom):
    return atom["context"].get("locus.trace")
