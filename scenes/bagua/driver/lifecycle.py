from pathlib import Path
import signal

from host import alive, emit, sessions
from terminal import Terminal
from wire import geometry, wait
from workspace import join


def disconnected(host):
    owner = host.wire()
    session, pane = host.new(owner)
    pid = host.start(owner, pane)
    owner.close()
    monitor = host.wire()
    join(monitor, session)
    wait(lambda: sessions(monitor)[0]["control"]["owner"] is None)
    assert alive(pid) and host.process.poll() is None
    view = Terminal(host, ["connect", str(session), "--take", "--fixed"], (20, 6))
    try:
        wait(lambda: sessions(monitor)[0]["control"]["owner"] is not None)
        view.process.send_signal(signal.SIGKILL)
        view.process.wait(timeout=5)
        wait(lambda: sessions(monitor)[0]["control"]["owner"] is None)
        assert alive(pid) and host.process.poll() is None
        monitor.workspace({"Acquire": {"session": session, "fixed": None}})
        monitor.ask({"Send": {"pane": pane, "input": {"Text": "AFTER-CLIENT-DEATH"}}})
        wait(lambda: b"AFTER-CLIENT-DEATH" in (host.root / "input.bin").read_bytes())
        emit("client disconnect and SIGKILL preserve host/application; control recovers", {"session": session, "pid": pid})
    finally:
        view.close()


def outcomes(host):
    client = host.wire()
    session, pane = host.new(client)
    pid = host.start(client, pane, "exit")
    wait(lambda: client.process(pane)["exited"])
    state = client.process(pane)
    assert state["code"] == 23 and state["signal"] is None
    assert not alive(pid)
    assert str(pane) in next(iter(sessions(client)[-1]["windows"].values()))["panes"]
    client.workspace({"Close": {"Session": session}})
    session, pane = host.new(client)
    pid = host.start(client, pane, "eof")
    wait(lambda: client.process(pane)["eof"])
    assert not client.process(pane)["exited"] and alive(pid)
    client.workspace({"Close": {"Pane": pane}})
    assert not alive(pid)
    state = sessions(client)[-1]
    assert state["id"] == session and state["windows"] == {}
    client.workspace({"Close": {"Session": session}})
    emit("exit retains pane; EOF differs from exit; last-pane close retains empty session", True)


def retirement(host):
    client = host.wire()
    session, pane = host.new(client)
    pid = host.start(client, pane, "stubborn")
    wait(lambda: (host.root / "ready").exists())
    client.workspace({"Close": {"Pane": pane}})
    assert not alive(pid)
    client.ask({"Inspect": pane}, "missing")
    client.workspace({"Close": {"Session": session}})
    session, pane = host.new(client)
    program = host.program("echo")
    program["executable"] = "/definitely-missing-bagua-scene-program"
    client.ask({"Start": {"pane": pane, "program": program}}, "io")
    state = sessions(client)[-1]
    assert next(iter(next(iter(state["windows"].values()))["panes"].values()))["state"] == "Failed"
    emit("retirement finishes after actual reap; retired IDs refuse; launch failure remains observable", True)


def run(host):
    disconnected(host)
    outcomes(host)
    retirement(host)
