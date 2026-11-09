import os
from pathlib import Path
import shutil
import signal
import time
from types import SimpleNamespace

from host import emit, sessions
from terminal import Terminal
from wire import wait
from .common import active, command, config, panes, profile, settle, state, until


def describe(client, pane):
    messages = client.ask({"Describe": pane})
    return next(value["Context"]["state"] for value in messages if "Context" in value)


def shell(host):
    return ["--executable", "/bin/sh", "--cwd", str(host.root), "--arg=-i", "--env", "PATH=/usr/bin:/bin", "--env", "TERM=xterm-256color"]


def cwd(host):
    settings = config(host, profile())
    view = Terminal(host, [*settings, "enter", "workspace", *shell(host)], (160, 25))
    client = host.wire()
    try:
        until(view, "control")
        first = state(client)
        a = host.root / "cwd-a"
        b = host.root / "cwd-b"
        a.mkdir()
        b.mkdir()
        view.write(f"cd '{a}'; printf '\\103\\127\\104-READY\\n'\r".encode())
        until(view, "CWD-READY")
        view.write(b"\x11\\printf '\\123\\120\\114\\111\\124-READY\\n'\r")
        wait(lambda: len(panes(state(client))) == 2)
        until(view, "SPLIT-READY")
        selected = active(client)
        from workspace import join
        join(client, first["id"])
        assert describe(client, selected)["cwd"] == str(a)
        other = next(pane["id"] for pane in panes(state(client)) if pane["id"] != selected)
        from wire import text
        assert "SPLIT-READY" not in text(client.screen(other))
        view.write(f"cd '{b}'; printf '\\116\\105\\130\\124-READY\\n'\r".encode())
        until(view, "NEXT-READY")
        view.write(b"\x11=")
        wait(lambda: len(state(client)["windows"]) == 2)
        wait(lambda: all(pane["state"] == "Running" for pane in panes(state(client))))
        assert describe(client, active(client))["cwd"] == str(b)
        view.write(b"\x1b1")
        wait(lambda: state(client)["selected"] == first["selected"])
        view.write(f"rmdir '{b}'; printf '\\104\\105\\114-READY\\n'\r".encode())
        until(view, "DEL-READY")
        before = state(client)
        view.write(b"\x11-NOT-ADMITTED")
        until(view, "Inspect")
        assert len(panes(state(client))) == len(panes(before))
        assert all("NOT-ADMITTED" not in text(client.screen(pane["id"])) for pane in panes(state(client)))
        view.write(b"printf '\\103\\127\\104_FRESH\\n'\r")
        until(view, "CWD_FRESH")
        view.detach(b"\x1bq")
        client.workspace({"Acquire": {"session": first["id"], "fixed": None}})
        client.workspace({"Close": {"Session": first["id"]}})
        emit("foreground cwd follows shell cd for split/window; batched input targets new pane; deleted cwd refuses", True)
    finally:
        view.close()


def finished(view, code):
    wait(lambda: (view.drain(), view.process.poll() is not None)[1])
    assert view.process.wait(timeout=1) == code, bytes(view.output)
    import termios
    assert termios.tcgetattr(view.slave) == view.before


def server(guest, path):
    expected = str(path).encode()
    found = []
    for item in Path("/proc").iterdir():
        if not item.name.isdecimal():
            continue
        try:
            args = (item / "cmdline").read_bytes().split(b"\0")
            if {expected, b"serve", b"--orphan"}.issubset(args):
                assert os.stat(item / "exe").st_ino == os.stat(guest.binary).st_ino
                found.append(int(item.name))
        except FileNotFoundError:
            pass
    assert len(found) == 1, found
    return found[0]


def cold(host):
    root = host.root / "entry"
    root.mkdir(mode=0o700)
    runtime = root / "runtime"
    runtime.mkdir(mode=0o700)
    guest = SimpleNamespace(binary=host.binary, path=None, env=dict(host.env, XDG_RUNTIME_DIR=str(runtime)), root=root)
    path = runtime / "bagua/host.sock"
    absent = Terminal.launch(guest, ["--no-config", "enter", "--observe"])
    try:
        finished(absent, 2)
        assert not path.parent.exists()
    finally:
        absent.close()
    path.parent.mkdir(mode=0o700)
    path.write_text("owned sentinel")
    inode = path.stat().st_ino
    unknown = Terminal.launch(guest, ["--no-config"])
    try:
        finished(unknown, 2)
        assert path.read_text() == "owned sentinel" and path.stat().st_ino == inode
    finally:
        unknown.close()
        path.unlink()
    racing(host, guest, path)


def racing(host, guest, path):
    views = [Terminal.launch(guest, ["--no-config"]) for _ in range(2)]
    pid = None
    client = None
    try:
        def settled():
            for view in views:
                view.drain()
            return sum("[t] Take" in view.screen() for view in views) == 1 and sum("bagua w1 control" in view.screen() for view in views) == 1
        wait(settled)
        loser = next(view for view in views if "[t] Take" in view.screen())
        loser.write(b"q")
        finished(loser, 0)
        pid = server(guest, path)
        from wire import Wire
        client = Wire(path)
        wait(lambda: all(pane["state"] == "Running" for value in sessions(client) for pane in panes(value)))
        values = sessions(client)
        assert len(values) == 1 and values[0]["name"] == "workspace"
        assert len(values[0]["windows"]) == 1
        assert path.parent.stat().st_mode & 0o777 == 0o700
        from workspace import join
        join(client, values[0]["id"])
        child = describe(client, panes(values[0])[0]["id"])["pid"]
        winner = next(view for view in views if view.process.poll() is None)
        winner.detach()
        assert Path(f"/proc/{pid}").exists()
        guest.path = path
        result = command(guest, ["--no-config", "--socket", str(path), "shutdown"])
        assert result.returncode == 0, result.stderr
        wait(lambda: not path.exists())
        wait(lambda: os.waitpid(pid, os.WNOHANG)[0] == pid)
        pid = None
        assert not Path(f"/proc/{child}").exists()
        emit("default endpoint self-start race creates one exact workspace; observer missing/unknown endpoint refuse without deletion", True)
    finally:
        if client is not None:
            client.close()
        for view in views:
            view.close()
        if pid is not None and Path(f"/proc/{pid}").exists():
            os.kill(pid, signal.SIGTERM)
            os.waitpid(pid, 0)


def identity(host):
    duplicate = host.root / "other-bagua"
    shutil.copyfile(host.binary, duplicate)
    duplicate.chmod(0o700)
    guest = SimpleNamespace(binary=str(duplicate), path=host.path, env=host.env)
    view = Terminal.launch(guest, ["--no-config", "enter"])
    try:
        finished(view, 2)
        assert b"different executable file" in view.output
        assert host.path.exists()
        emit("same bytes in different executable inode refuse existing host identity", True)
    finally:
        view.close()


def run(host):
    identity(host)
    cold(host)
    cwd(host)
