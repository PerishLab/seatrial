from pathlib import Path
import json
import time

from host import emit, sessions
from terminal import Terminal
from wire import created, geometry, text, wait


def received(host):
    path = host.root / "input.bin"
    return path.read_bytes() if path.exists() else b""


def join(client, session, size=None):
    client.workspace({"Join": {"session": session, "viewport": size or geometry()}})


def control(host):
    owner = host.wire()
    session, pane = host.new(owner)
    host.start(owner, pane)
    wait(lambda: "READY" in text(owner.screen(pane)))
    observer = host.wire()
    join(observer, session, geometry(20, 6))
    observer.ask({"Send": {"pane": pane, "input": {"Text": "forbidden"}}}, "observer")
    observer.workspace({"Resize": {"session": session, "geometry": geometry(2, 1)}})
    assert sessions(owner)[0]["sizing"] == {"Automatic": geometry()}
    assert b"forbidden" not in received(host)
    owner.workspace({"Resize": {"session": session, "geometry": geometry(70, 14)}})
    wait(lambda: json.loads((host.root / "echo.size").read_text()) == geometry(70, 14))
    split = owner.workspace({"Split": {"pane": pane, "split": {"axis": "Columns", "ratio": 500}}})
    other = created(split, "Pane")
    owner.workspace({"Focus": pane})
    owner.workspace({"Transfer": {"session": session, "next": observer.hello["client"]}}, "geometry")
    assert sessions(owner)[0]["control"]["owner"] == owner.hello["client"]
    observer.workspace({"Resize": {"session": session, "geometry": geometry(30, 8)}})
    owner.workspace({"Transfer": {"session": session, "next": observer.hello["client"]}})
    owner.workspace({"Focus": pane}, "observer")
    observer.workspace({"Size": {"session": session, "sizing": {"Fixed": geometry(60, 12)}}})
    observer.workspace({"Resize": {"session": session, "geometry": geometry(40, 10)}})
    assert sessions(observer)[0]["sizing"] == {"Fixed": geometry(60, 12)}
    observer.workspace({"Size": {"session": session, "sizing": {"Automatic": geometry(40, 10)}}})
    window = observer.workspace({"Window": session})
    state = sessions(observer)[0]
    assert state["selected"] == created(window, "Window") and len(state["windows"]) == 2
    observer.workspace({"Close": {"Pane": other}})
    observer.workspace({"Close": {"Session": session}})
    emit("observer, transfer, geometry, split, focus, window and close", True)


def launch(host):
    program = host.program("echo")
    args = ["open", "--executable", program["executable"], "--cwd", program["cwd"]]
    for arg in program["args"]:
        args.append("--arg=" + arg)
    for name, value in program["environment"].items():
        args.extend(["--env", name + "=" + value])
    return args


def interactive(host):
    view = Terminal(host, launch(host))
    monitor = host.wire()
    observer = None
    try:
        state = wait(lambda: sessions(monitor))[-1]
        session = state["id"]
        join(monitor, session)
        pane = next(iter(next(iter(state["windows"].values()))["panes"].values()))["id"]
        wait(lambda: "READY" in text(monitor.screen(pane)))
        wait(lambda: b"READY" in view.visible())
        assert b"bagua-outer-secret" not in view.output
        view.write(b"\x07[" + b"\x1b[D" * 5 + b"\x07c\x07p")
        wait(lambda: b"\x1b[200~READY\x1b[201~" in received(host))
        view.write(b"hello\x1b[200~paste\x1b[201~\x1b[A")
        wait(lambda: b"hello\x1b[200~paste\x1b[201~\x1b[A" in received(host))
        view.write(b"\x1b[<0;2;2M")
        wait(lambda: b"\x1b[<0;2;2M" in received(host))
        view.write(b"\x07v")
        wait(lambda: len(next(iter(sessions(monitor)[-1]["windows"].values()))["panes"]) == 2)
        view.write(b"\x07n")
        wait(lambda: len(sessions(monitor)[-1]["windows"]) == 2)
        panes = [pane["id"] for window in sessions(monitor)[-1]["windows"].values() for pane in window["panes"].values()]
        wait(lambda: all("READY" in text(monitor.screen(target)) for target in panes))
        view.write(b"\x07w\x07\t")
        observer = Terminal(host, ["connect", str(session)], (12, 4))
        peers = wait(lambda: [int(key) for key in sessions(monitor)[-1]["clients"] if int(key) not in (monitor.hello["client"], state["control"]["owner"])])
        assert len(peers) == 1
        peer = peers[0]
        observer.write(b"observer-must-not-write")
        time.sleep(0.1)
        assert b"observer-must-not-write" not in received(host)
        view.detach()
        assert host.process.poll() is None
        wait(lambda: sessions(monitor)[-1]["control"]["owner"] is None)
        observer.resize(30, 8)
        observer.write(b"\x07a")
        wait(lambda: sessions(monitor)[-1]["control"]["owner"] == peer)
        wait(lambda: f"bagua s{session} c{peer} control".encode() in observer.visible())
        observer.write(b"acquired")
        try:
            wait(lambda: b"acquired" in received(host))
        except AssertionError:
            emit("interactive failure diagnostics", {"catalog": monitor.catalog(), "output": repr(observer.visible()[-4096:]), "input": repr(received(host)[-4096:])})
            raise
        saved = monitor.screen(pane)
        observer.detach()
        current = monitor.screen(pane)
        assert current["cells"] == saved["cells"] and current["extent"] == saved["extent"]
        assert host.cli(["list", "--json"])
        emit("interactive input, paste, mouse, copy, observer, detach, reacquire and TTY restore", True)
    finally:
        view.close()
        if observer is not None:
            observer.close()


def run(host):
    control(host)
    interactive(host)
