import json
import signal
from types import SimpleNamespace

from host import alive, emit, sessions
from terminal import Terminal
from wire import created, geometry, text, wait
from workspace import join
from interaction.proxy import Proxy
from .common import active, legacy as config, launch, panes, profile, state, until
from .entry import describe, finished, shell


def absent(client, session):
    return all(value["id"] != session for value in sessions(client))


def setup(host, client, pane, settings):
    (host.root / "departure.json").write_text(json.dumps(settings))
    pid = host.start(client, pane, "departure")
    if settings.get("eof"):
        wait(lambda: client.process(pane)["eof"])
        assert not client.process(pane)["exited"] and alive(pid)
    else:
        wait(lambda: client.screen(pane)["bytes"] == len(b"DEPARTURE-READY") + 131072)
        assert "TAIL" in text(client.screen(pane))
    return pid


def release(host, pid):
    (host.root / ("release-" + str(pid))).touch()


def outcomes(host):
    for settings in [{"code": 0}, {"code": 7}, {"signal": True}, {"eof": True}]:
        client = host.wire()
        session, pane = host.new(client)
        client.ask({"Subscribe": {"target": {"Session": session}, "after": None}})
        pid = setup(host, client, pane, settings)
        assert next(value for value in sessions(client) if value["id"] == session)["windows"]
        release(host, pid)
        wait(lambda: absent(client, session))
        outcome = next(value for value in reversed(client.events) if value["pane"] == pane and value["exited"])
        assert outcome["signal"] == (signal.SIGTERM if settings.get("signal") else None)
        assert outcome["code"] == (None if settings.get("signal") else settings.get("code", 0))
        assert not alive(pid)
        client.ask({"Inspect": pane}, "missing")
        client.close()
    emit("normal/nonzero/signal exit closes after full tail consumption and reap; EOF alone preserves live pane", True)


def background(host):
    client = host.wire()
    session, pane = host.new(client)
    pid = setup(host, client, pane, {})
    client.close()
    monitor = host.wire()
    wait(lambda: next(value for value in sessions(monitor) if value["id"] == session)["control"]["owner"] is None)
    assert alive(pid)
    release(host, pid)
    wait(lambda: absent(monitor, session))
    assert not alive(pid) and host.process.poll() is None
    emit("host retires background exit with no joined client or controller", True)


def layout(host):
    owner = host.wire()
    session, first = host.new(owner)
    survivor = host.start(owner, first)
    command = {"Split": {"pane": first, "split": {"axis": "Columns", "ratio": 500}}}
    pane = created(owner.workspace(command), "Pane")
    pid = setup(host, owner, pane, {})
    release(host, pid)
    wait(lambda: all(value["id"] != pane for value in panes(state(owner))))
    value = next(value for value in sessions(owner) if value["id"] == session)
    window = next(iter(value["windows"].values()))
    assert window["focus"] == first and alive(survivor) and not alive(pid)
    wait(lambda: json.loads((host.root / "echo.size").read_text()) == geometry())
    emit("exited split collapses, surviving focus and actual PTY geometry recover", True)
    return session, survivor, owner


def interactive(host, survivor, settings=None):
    settings = settings or config(host, profile())
    args = [*settings, "enter", "departure", *shell(host)]
    view = Terminal(host, args)
    client = host.wire()
    observer = None
    again = None
    try:
        until(view, "control")
        value = next(value for value in sessions(client) if value["name"] == "departure")
        session = value["id"]
        join(client, session)
        original = describe(client, panes(value)[0]["id"])["pid"]
        host.children.append(original)
        observer = Terminal(host, [*settings, "enter", "departure", "--observe"])
        until(observer, "observe")
        view.write(b"\x11\\")
        wait(lambda: len(panes(state(client))) == 2)
        wait(lambda: all(value["state"] == "Running" for value in panes(state(client))))
        view.write(b"printf '\\123PLIT_READY\\n'\r")
        until(view, "SPLIT_READY")
        view.write(b"exit\r")
        wait(lambda: len(panes(state(client))) == 1)
        wait(lambda: "SPLIT_READY" not in view.screen())
        view.write(b"\x11=")
        wait(lambda: len(state(client)["windows"]) == 2)
        wait(lambda: all(value["state"] == "Running" for value in panes(state(client))))
        view.write(b"printf '\\127INDOW_READY\\n'\r")
        until(view, "WINDOW_READY")
        view.write(b"exit 9\r")
        wait(lambda: len(state(client)["windows"]) == 1)
        wait(lambda: "WINDOW_READY" not in view.screen())
        view.write(b"printf '\\101LIVE\\n'\r")
        until(view, "ALIVE")
        view.write(b"exit\r")
        finished(view, 0)
        finished(observer, 0)
        assert absent(client, session) and not alive(original)
        assert alive(survivor) and host.process.poll() is None
        again = Terminal(host, args)
        until(again, "control")
        fresh = next(value for value in sessions(client) if value["name"] == "departure")
        assert fresh["id"] > session
        join(client, fresh["id"])
        pid = describe(client, panes(fresh)[0]["id"])["pid"]
        host.children.append(pid)
        assert alive(pid) and pid != original
        again.write(b"exit\r")
        finished(again, 0)
        assert absent(client, fresh["id"]) and not alive(pid)
        emit("shell exit collapses panes/windows; final exit restores controller and observer TTY; same-name reentry is fresh; other app survives", True)
    finally:
        view.close()
        if observer is not None:
            observer.close()
        if again is not None:
            again.close()


def run(host):
    outcomes(host)
    background(host)
    session, survivor, owner = layout(host)
    interactive(host, survivor)
    owner.workspace({"Close": {"Session": session}})
    assert not alive(survivor)
    metadata(host)


def metadata(host, settings=None):
    settings = settings or config(host, profile() + '\n[header]\nenabled=true\ncomponents=["pane","command","cwd"]\n', "metadata.toml")
    (host.root / "departure.json").write_text(json.dumps({"code": 0}))
    (host.root / "departure.pid").unlink(missing_ok=True)
    proxy = Proxy(host, "metadata")
    guest = SimpleNamespace(binary=host.binary, path=proxy.path, env=host.env)
    view = None
    monitor = host.wire()
    try:
        view = Terminal.launch(guest, launch(host, "departure", settings), (160, 40))
        wait(lambda: (view.drain(), proxy.held.is_set())[1])
        pid = wait(lambda: host.pid("departure"))
        host.children.append(pid)
        release(host, pid)
        wait(lambda: not sessions(monitor))
        assert not alive(pid)
        proxy.resume()
        finished(view, 0)
        assert proxy.refused.is_set()
        assert host.process.poll() is None and not sessions(monitor)
        emit("header metadata Describe receives actual immediate Missing without Finished after retirement; client exits zero with restored TTY", {"request": proxy.target, "pid": pid})
    finally:
        if view is not None:
            view.close()
        proxy.close()
