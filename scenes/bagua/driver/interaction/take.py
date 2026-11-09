from pathlib import Path

from daily.common import until
from daily.entry import describe, finished
from host import emit
from wire import wait
from .common import Trial


def success(host):
    trial = Trial(host, "interaction-take", "shell")
    try:
        trial.view.write(b"bagua_cookie=Kept; export bagua_cookie; printf '\\117\\127\\116\\105\\122_READY\\n'\r")
        until(trial.view, "OWNER_READY")
        old = trial.owner()
        next = trial.terminal([*trial.settings, "--take", "enter", trial.name])
        until(next, "control")
        finished(trial.view, 0)
        assert trial.owner() is not None and trial.owner() != old
        assert describe(trial.client, trial.pane())["pid"] == trial.pid
        next.write(b"printf '\\123\\125\\122\\126\\111\\126\\105\\104_%s\\n' \"$bagua_cookie\"\r")
        until(next, "SURVIVED_Kept")
        assert host.process.poll() is None and Path(f"/proc/{trial.pid}").exists()
        assert len(trial.state()["clients"]) == 2
        next.detach(b"\x1bq")
        emit("explicit top-level takeover leaves exactly one controller, restores old TTY and preserves the same shell PID and state", True)
    finally:
        trial.close()


def choices(host):
    trial = Trial(host, "interaction-choices", "shell")
    try:
        old = trial.owner()
        observer = trial.terminal([*trial.settings, "enter", trial.name])
        until(observer, "[t] Take")
        before = len(trial.state()["clients"])
        observer.write(b"o")
        wait(lambda: len(trial.state()["clients"]) == before + 1)
        wait(lambda: "[t] Take" not in observer.screen())
        until(observer, "observe")
        observer.write(b"x")
        until(observer, "requires the session controller")
        assert trial.owner() == old
        observer.detach(b"\x1bq")
        cancelled = trial.terminal([*trial.settings, "enter", trial.name])
        until(cancelled, "[t] Take")
        cancelled.write(b"q")
        finished(cancelled, 0)
        assert trial.owner() == old
        first = trial.terminal([*trial.settings, "enter", trial.name])
        second = trial.terminal([*trial.settings, "enter", trial.name])
        until(first, "[t] Take")
        until(second, "[t] Take")
        first.write(b"t")
        until(first, "control")
        finished(trial.view, 0)
        current = trial.owner()
        second.write(b"t")
        finished(second, 2)
        assert trial.owner() == current
        assert b"controller changed" in second.output
        first.detach(b"\x1bq")
        emit("occupied entry offers observe/cancel; two observed-owner choices cannot displace a newer winner", True)
    finally:
        trial.close()


def refusal(host):
    trial = Trial(host, "interaction-refusal", "shell")
    try:
        trial.view.write(b"\x11-printf '\\124\\127\\117_READY\\n'\r")
        until(trial.view, "TWO_READY")
        old = trial.owner()
        pane = trial.pane()
        pid = describe(trial.client, pane)["pid"]
        size = trial.state()["sizing"]
        refused = trial.terminal([*trial.settings, "enter", trial.name, "--take"], (40, 3))
        finished(refused, 2)
        assert trial.owner() == old and trial.state()["sizing"] == size
        assert describe(trial.client, pane)["pid"] == pid
        trial.view.write(b"printf '\\117\\127\\116\\105\\122_STILL_OK\\n'\r")
        until(trial.view, "OWNER_STILL_OK")
        fixed = trial.terminal([*trial.settings, "connect", str(trial.session), "--take", "--fixed"], (40, 3))
        until(fixed, "control")
        finished(trial.view, 0)
        assert trial.state()["sizing"] == {"Fixed": next(iter(size.values()))}
        assert trial.owner() is not None and trial.owner() != old
        assert describe(trial.client, pane)["pid"] == pid
        fixed.detach(b"\x1bq")
        emit("invalid takeover preserves controller/canvas/apps; explicit fixed connect takeover preserves canvas even on a tiny viewport", True)
    finally:
        trial.close()


def run(host):
    success(host)
    choices(host)
    refusal(host)
