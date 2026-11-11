import array
import fcntl
from pathlib import Path
import termios
import time
from types import SimpleNamespace

from daily.common import config, profile, settle, until
from daily.entry import describe, finished
from host import emit
from terminal import Terminal
from wire import text, wait
from .common import Trial, template
from .proxy import Proxy


def empty(view):
    count = array.array("i", [0])
    fcntl.ioctl(view.slave, termios.FIONREAD, count, True)
    return count[0] == 0


def connect(trial, proxy):
    guest = SimpleNamespace(binary=trial.host.binary, path=proxy.path, env=trial.host.env)
    settings = config(trial.host, profile() + "[header]\nenabled=false\nformat=''\n[status]\n[[status.lines]]\nleft=''\n", "recovery-empty.toml")
    view = Terminal.launch(guest, [*settings, "connect", str(trial.session), "--take", *template(trial.host, "untracked")], (120, 28))
    trial.views.append(view)
    until(view, "READY")
    wait(lambda: trial.owner() is not None)
    return view


def fragmented(host):
    trial = Trial(host, "interaction-fragment")
    proxy = Proxy(host, "refuse")
    try:
        trial.view.detach(b"\x1bq")
        view = connect(trial, proxy)
        pane = trial.pane()
        view.resize(120, 3)
        wait(lambda: trial.state()["sizing"]["Automatic"]["rows"] == 2)
        view.write(b"\x11-")
        wait(proxy.held.is_set)
        view.write(b"\x1b[200~FRAGMENT-NEVER")
        wait(lambda: empty(view))
        proxy.resume()
        until(view, "Split top/bottom:")
        for byte in b"-OLD\x1b[201~FRAGMENT-FRESH":
            view.write(bytes([byte]))
        wait(lambda: "FRAGMENT-FRESH" in text(trial.client.screen(pane)))
        assert "FRAGMENT-NEVER" not in text(trial.client.screen(pane))
        assert "-OLD" not in text(trial.client.screen(pane))
        view.detach(b"\x1bq")
        emit("a real refused split held at the public wire boundary invalidates an already-read partial paste across later TTY fragments", True)
    finally:
        proxy.close()
        trial.close()


def pending(host, bounded):
    trial = Trial(host, "interaction-bounded" if bounded else "interaction-resume")
    proxy = Proxy(host, "recover")
    try:
        trial.view.write(b"\x11=SECOND-READY")
        wait(lambda: len(trial.state()["windows"]) == 2)
        second = trial.pane()
        wait(lambda: "SECOND-READY" in text(trial.client.screen(second)))
        first = next(pane for pane in trial.bodies() if pane != second)
        pid = describe(trial.client, second)["pid"]
        trial.view.detach(b"\x1bq")
        view = connect(trial, proxy)
        view.write(b"\x1b1")
        wait(proxy.held.is_set)
        until(view, "Recovering")
        view.write(b"\x1b[27uUNCERTAIN-NEVER")
        until(view, "input was not admitted")
        assert all("UNCERTAIN-NEVER" not in body for body in trial.bodies().values())
        if bounded:
            expired(view)
            finished(view, 2)
            assert host.process.poll() is None and host.path.exists()
            wait(lambda: trial.owner() is None)
            assert Path(f"/proc/{pid}").exists()
            assert describe(trial.client, second)["pid"] == pid
        else:
            proxy.resume()
            until(view, "Live; uncertain input discarded")
            view.write(b"RECOVERY-FRESH")
            wait(lambda: "RECOVERY-FRESH" in text(trial.client.screen(first)))
            assert "RECOVERY-FRESH" not in text(trial.client.screen(second))
            view.detach(b"\x1bq")
        assert all("UNCERTAIN-NEVER" not in body for body in trial.bodies().values())
        emit("unknown focus never replays or admits Escape-unlocked input; bounded client exit preserves host/apps" if bounded else "explicit catalog completion recovers unknown focus for fresh targeted input without replay", True)
    finally:
        proxy.close()
        trial.close()


def expired(view):
    deadline = time.monotonic() + 2.2
    while time.monotonic() < deadline:
        view.drain()
        time.sleep(0.01)
    assert view.process.poll() is None
    assert "Recovering" in view.screen()


def run(host):
    fragmented(host)
    pending(host, False)
    pending(host, True)
