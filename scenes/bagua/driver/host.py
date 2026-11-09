import json
import os
from pathlib import Path
import signal
import subprocess

from wire import Wire, created, geometry, wait


class Host:
    def __init__(self, root, binary, source):
        self.root = Path(root)
        self.root.mkdir(mode=0o700, exist_ok=True)
        self.root.chmod(0o700)
        self.binary = str(Path(binary).resolve())
        self.source = Path(source)
        self.path = self.root / "host.sock"
        self.home = self.root / "home"
        self.home.mkdir(mode=0o700)
        self.env = {"PATH": "/usr/bin:/bin", "HOME": str(self.home), "LANG": "C.UTF-8", "TERM": "xterm-256color"}
        self.clients = []
        self.children = []
        self.log = (self.root / "host.log").open("xb")
        config = self.root / "server.toml"
        settings = ["--config", str(config)] if config.exists() else ["--no-config"]
        self.process = subprocess.Popen([self.binary, *settings, "--socket", str(self.path), "serve"], env=self.env, stdout=self.log, stderr=self.log)
        wait(lambda: self.path.exists() or self.process.poll() is not None)
        assert self.process.poll() is None
        assert self.path.stat().st_mode & 0o777 == 0o600

    def wire(self):
        client = Wire(self.path)
        self.clients.append(client)
        return client

    def new(self, client, size=None):
        messages = client.workspace({"Create": size or geometry()})
        session = created(messages, "Session")
        pane = created(messages, "Pane")
        client.workspace({"Join": {"session": session, "viewport": size or geometry()}})
        client.workspace({"Acquire": {"session": session, "fixed": None}})
        return session, pane

    def program(self, mode):
        return {"executable": "/usr/bin/python3", "args": ["-B", "-u", str(self.source / "program.py"), mode, str(self.root)], "cwd": str(self.root), "environment": self.env}

    def start(self, client, pane, mode="echo"):
        (self.root / (mode + ".pid")).unlink(missing_ok=True)
        client.ask({"Start": {"pane": pane, "program": self.program(mode)}})
        pid = wait(lambda: self.pid(mode))
        self.children.append(pid)
        return pid

    def pid(self, mode):
        path = self.root / (mode + ".pid")
        if path.exists() and path.read_text().strip():
            return int(path.read_text())
        return None

    def cli(self, args):
        result = subprocess.run([self.binary, "--socket", str(self.path), *args], env=self.env, capture_output=True, text=True, timeout=10)
        assert result.returncode == 0, result.stderr
        return result.stdout

    def stop(self):
        if self.process.poll() is None:
            self.cli(["shutdown"])
        self.process.wait(timeout=8)
        assert self.process.returncode == 0
        assert not self.path.exists()
        for pid in self.children:
            assert not Path(f"/proc/{pid}").exists(), f"owned child {pid} survived shutdown"
        for path in self.root.glob("*.pid"):
            assert not alive(int(path.read_text())), path.name

    def close(self):
        for client in self.clients:
            client.close()
        if self.process.poll() is None:
            self.process.send_signal(signal.SIGTERM)
            self.process.wait(timeout=8)
        self.log.close()


def alive(pid):
    return Path(f"/proc/{pid}").exists()


def sessions(client):
    return client.catalog()["sessions"]


def emit(name, evidence):
    print(json.dumps({"assertion": name, "evidence": evidence}, sort_keys=True), flush=True)
