import base64

from host import emit
from terminal import Terminal
from wire import geometry, text, wait
from workspace import join
from .common import active, legacy as config, launch, panes, profile, settle, state, until


def copied(view):
    view.drain()
    return [value for value in view.view.osc if value.startswith("52;c;")]


def drag(view, end):
    x, y = end
    view.write(f"\x1b[<0;1;1M\x1b[<32;{x};{y}M\x1b[<0;{x};{y}m".encode())


def copy(host, sample):
    target = sample["target"]
    content = sample["source"]
    expected = sample["text"]
    path = host.root / "clipboard.txt"
    path.write_text(content)
    (host.root / "clipboard.ready").unlink(missing_ok=True)
    settings = config(host, profile() + f"[clipboard]\ntarget='{target}'\n")
    view = Terminal(host, launch(host, "clipboard", settings), sample["size"])
    client = host.wire()
    try:
        wait(lambda: (host.root / "clipboard.ready").exists())
        ready, count = sample["ready"]
        wait(lambda: view.screen().count(ready) == count)
        value = state(client)
        assert not copied(view)
        drag(view, sample["end"])
        until(view, f"Copied {len(expected.encode())} bytes locally")
        expected = "52;c;" + base64.b64encode(expected.encode()).decode()
        if target == "osc52":
            wait(lambda: copied(view) == [expected])
        else:
            assert not copied(view)
        view.detach(b"\x1bq")
        assert copied(view) == ([expected] if target == "osc52" else [])
        join(client, value["id"])
        client.workspace({"Acquire": {"session": value["id"], "fixed": None}})
        client.workspace({"Close": {"Session": value["id"]}})
        emit("user drag-release copy policy, complete OSC52 base64/ST, no application OSC passthrough", {"target": target, "bytes": len(sample["text"].encode())})
    finally:
        view.close()


def manual(host):
    (host.root / "clipboard.txt").write_text("hello")
    original = profile() + "[clipboard]\ntarget='osc52'\n[mouse]\ncopy_on_release=false\n"
    settings = config(host, original)
    view = Terminal(host, launch(host, "clipboard", settings))
    client = host.wire()
    try:
        until(view, "hello")
        value = state(client)
        drag(view, (5, 1))
        settle(view)
        assert not copied(view)
        view.write(b"\x11c")
        wait(lambda: copied(view) == ["52;c;aGVsbG8="])
        path = host.root / "client.toml"
        config(host, original.replace("target='osc52'", "target='osc52'\nlimit=4").replace("copy_on_release=false", "copy_on_release=true"))
        view.write(b"\x11r")
        until(view, "Configuration reloaded")
        drag(view, (5, 1))
        until(view, "protocol resource limit exceeded")
        assert copied(view) == ["52;c;aGVsbG8="]
        view.detach(b"\x1bq")
        join(client, value["id"])
        client.workspace({"Acquire": {"session": value["id"], "fixed": None}})
        client.workspace({"Close": {"Session": value["id"]}})
        emit("manual copy does not emit on release; reload enforces byte limit without extra OSC52", True)
    finally:
        view.close()


def divider(host):
    view = Terminal(host, launch(host, "untracked", config(host, profile())), (80, 24))
    client = host.wire()
    observer = None
    try:
        until(view, "READY")
        value = state(client)
        join(client, value["id"])
        first = active(client)
        view.write(b"\x11\\")
        wait(lambda: len(panes(state(client))) == 2)
        second = active(client)
        wait(lambda: all(pane["state"] == "Running" for pane in panes(state(client))))
        view.write(b"\x1b[<0;40;2M\x1b[<32;51;2M\x1b[<0;51;2m")
        wait(lambda: client.screen(first)["extent"]["width"] == 50)
        assert client.screen(second)["extent"]["width"] == 29
        observer = Terminal(host, [*config(host, "", "observer-legacy.toml"), "connect", str(value["id"])], (80, 24))
        before = state(client)
        observer.write(b"\x1b[<0;51;2M\x1b[<32;60;2M\x1b[<0;60;2m")
        settle(observer)
        assert state(client)["windows"] == before["windows"]
        observer.detach()
        view.write(b"\x1b[<0;2;2MFOCUSED-FIRST")
        wait(lambda: active(client) == first)
        wait(lambda: "FOCUSED-FIRST" in text(client.screen(first)))
        assert "FOCUSED-FIRST" not in text(client.screen(second))
        view.write(b"\x1b[<0;51;2M\x1b[<32;1;2M\x1b[<0;1;2m")
        settle(view)
        assert client.screen(first)["extent"]["width"] >= 2
        assert client.screen(second)["extent"]["width"] >= 2
        view.detach(b"\x1bq")
        client.workspace({"Acquire": {"session": value["id"], "fixed": None}})
        client.workspace({"Close": {"Session": value["id"]}})
        emit("controller divider drag changes real PTY widths; observer cannot mutate; click focus routes text; minimum geometry preserved", True)
    finally:
        view.close()
        if observer is not None:
            observer.close()


def run(host):
    samples = [
        {"target": "local", "source": "hello", "text": "hello", "size": (80, 25), "end": (5, 1), "ready": ("hello", 1)},
        {"target": "osc52", "source": "hello", "text": "hello", "size": (80, 25), "end": (5, 1), "ready": ("hello", 1)},
        {"target": "osc52", "source": "a\u0301" * 12800, "text": "a\u0301" * 12800, "size": (256, 65), "end": (256, 50), "ready": ("a\u0301", 12800)},
        {"target": "osc52", "source": "α界\r\nbeta", "text": "α界\nbeta", "size": (80, 25), "end": (4, 2), "ready": ("beta", 1)},
        {"target": "osc52", "source": "界", "text": "界", "size": (80, 25), "end": (2, 1), "ready": ("界", 1)},
    ]
    for sample in samples:
        copy(host, sample)
    manual(host)
    divider(host)
