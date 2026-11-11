import time

from host import Host, emit
from terminal import Terminal
from wire import geometry, text, wait
from workspace import join
from .common import legacy as config, data, send, settle, until


def page(client, pane, previous=None, top=None):
    read = {"ticket": 19, "back": 0, "scope": previous["scope"] if previous else None, "top": top, "column": 0}
    messages = client.ask({"History": {"pane": pane, "read": read}})
    value = next(message["History"]["state"] for message in messages if "History" in message)
    assert value["ticket"] == 19
    assert len(value["screen"]["cells"]) <= 512 * 256
    return value


def refuse(client, pane, read):
    request = client.send({"History": {"pane": pane, "read": read}})
    while True:
        message = client.read()
        reply = message.get("Reply", {})
        if reply.get("request") == request and "Err" in reply.get("result", {}):
            return reply["result"]["Err"]
        finished = message.get("Finished", {})
        if finished.get("request") == request:
            assert "Err" in finished["result"], finished
            return finished["result"]["Err"]


def rows(value):
    screen = value["screen"]
    width = screen["extent"]["width"]
    content = [cell["text"] or " " for cell in screen["cells"] if cell["width"] != 0]
    return ["".join(content[index:index + width]) for index in range(0, len(content), width)]


def completed(client, pane):
    deadline = time.monotonic() + 120
    while time.monotonic() < deadline:
        if "HISTORY-DONE" in text(client.screen(pane)):
            return
        time.sleep(0.03)
    raise AssertionError("history producer did not finish")


def capacity(host, width):
    client = host.wire()
    session, pane = host.new(client, geometry(width, 24))
    (host.root / "history.done").unlink(missing_ok=True)
    host.start(client, pane, "history")
    completed(client, pane)
    first = page(client, pane)
    assert first["next"] - first["first"] == 50000, first
    assert first["bytes"] <= 67108864 and first["evicted"] > 0
    top = first["first"]
    count = 0
    while top < first["next"]:
        value = page(client, pane, first, top)
        assert value["top"] == top and value["scope"] == first["scope"]
        retained = min(24, first["next"] - top)
        for offset, row in enumerate(rows(value)[:retained]):
            assert row == f"{top + offset:08d}:" + "x" * (width - 9), (top, offset, row)
        top += retained
        count += retained
    assert count == 50000
    boundary(host, client, (session, pane), first)
    emit("all 50000 physical full-width rows round-trip with bounded per-pane page", {"columns": width, "rows": count, "bytes": first["bytes"], "evicted": first["evicted"]})


def output(host, client, pane, value):
    from .program import replies
    before = client.screen(pane)["bytes"]
    send(client, pane, value)
    wait(lambda: data(host, "history.ack") == value.encode())
    wait(lambda: client.screen(pane)["bytes"] >= before + len(replies()[value.encode()]))


def boundary(host, client, target, first):
    session, pane = target
    stable = page(client, pane, first, first["first"])
    output(host, client, pane, "a")
    assert page(client, pane)["next"] == first["next"]
    resumed = host.wire()
    join(resumed, session, geometry(80, 24))
    assert page(resumed, pane, first, first["first"])["screen"]["cells"] == stable["screen"]["cells"]
    invalid = {"ticket": 1, "back": 0, "scope": dict(first["scope"], host=[0] * 16), "top": first["first"], "column": 0}
    refuse(client, pane, invalid)
    view = Terminal(host, [*config(host, "", "observer-legacy.toml"), "connect", str(session)], (widthof(first), 25))
    try:
        until(view, "HISTORY-DONE")
        view.write(b"\x07\x1b[5~")
        until(view, "History")
        frozen = view.screen().splitlines()[:-1]
        output(host, client, pane, "o")
        settle(view)
        assert view.screen().splitlines()[:-1] == frozen
        view.write(b"\x1b[<0;1;1M\x1b[<32;8;1M\x1b[<0;8;1m")
        until(view, "Copied 8 bytes locally")
        until(view, "EXTRA-ROW")
        view.write(b"\x1b[<64;2;2M")
        until(view, "History")
        view.write(b"\x1b[<65;2;2M")
        until(view, "Live")
        view.detach()
    finally:
        view.close()
    invalid["scope"] = first["scope"]
    refuse(client, pane, invalid)
    client.workspace({"Resize": {"session": session, "geometry": geometry(widthof(first), 23)}})
    invalid["top"] = page(client, pane)["first"]
    refuse(client, pane, invalid)
    output(host, client, pane, "c")
    refuse(client, pane, {"ticket": 1, "back": 0, "scope": None, "top": None, "column": 0})
    client.workspace({"Close": {"Session": session}})


def widthof(value):
    return value["screen"]["extent"]["width"]


def budget(host):
    root = host.root / "budget"
    root.mkdir()
    (root / "server.toml").write_text("[history]\nlines=50000\nbytes=65536\ntotal=65536\n")
    small = Host(root, host.binary, host.source)
    try:
        client = small.wire()
        session, pane = small.new(client)
        small.start(client, pane, "history")
        completed(client, pane)
        before = page(client, pane)
        assert 0 < before["bytes"] <= 65536 and before["next"] - before["first"] < 50000
        from wire import created
        other = created(client.workspace({"Split": {"pane": pane, "split": {"axis": "Columns", "ratio": 500}}}), "Pane")
        wait(lambda: page(client, pane)["bytes"] <= 32768)
        small.start(client, other, "history")
        completed(client, other)
        assert page(client, other)["bytes"] <= 32768
        assert page(client, pane)["bytes"] + page(client, other)["bytes"] <= 65536
        small.stop()
        emit("byte eviction and two-pane fair share use configured host total", True)
    finally:
        small.close()


def run(host):
    capacity(host, 80)
    capacity(host, 160)
    budget(host)
