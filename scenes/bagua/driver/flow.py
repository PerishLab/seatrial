import hashlib
import socket
import time

from host import emit, sessions
from terminal import Terminal
from wire import created, geometry, text, wait
from workspace import join
from daily.common import legacy


def output(host):
    owner = host.wire()
    session, pane = host.new(owner, geometry(256, 64))
    slow = host.wire()
    slow.socket.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 1024)
    join(slow, session)
    slow.ask({"Subscribe": {"target": {"Pane": pane}, "after": None}})
    host.start(owner, pane, "flood")
    wait(lambda: (host.root / "output.json").exists())
    wait(lambda: "FLOW-DONE" in text(owner.screen(pane)))
    screen = owner.screen(pane)
    assert screen["bells"] == 524288
    assert screen["bytes"] == 1048576 + len("你好FLOW-DONE".encode())
    assert "你好" in text(screen)
    assert len(screen["cells"]) == 256 * 64
    slow.close()
    resumed = host.wire()
    join(resumed, session)
    messages = resumed.ask({"Subscribe": {"target": {"Pane": pane}, "after": 0}})
    shown = [m["Screen"]["state"] for m in messages if "Screen" in m and m["Screen"]["pane"] == pane]
    full = shown[-1] if shown else resumed.view(pane)
    assert full["cells"] == screen["cells"] and full["revision"] == screen["revision"] and full["bytes"] == screen["bytes"]
    view = Terminal(host, [*legacy(host, "", "flow-legacy.toml"), "connect", str(session)], (3, 2))
    try:
        for columns, rows in [(1, 1), (2, 2), (8, 3), (3, 2)]:
            view.resize(columns, rows)
            time.sleep(0.04)
            view.drain()
            assert view.process.poll() is None
        view.detach()
    finally:
        view.close()
    owner.workspace({"Close": {"Session": session}})
    emit("flood bytes/bells, fragmented UTF-8, slow/lost observer, full resync and tiny clipping", {"bytes": screen["bytes"], "bells": screen["bells"]})


def input(host):
    owner = host.wire()
    session, pane = host.new(owner)
    host.start(owner, pane, "slow")
    wait(lambda: (host.root / "ready").exists())
    blocks = [chr(65 + index) * 4096 for index in range(8)]
    pending = {owner.send({"Send": {"pane": pane, "input": {"Text": block}}}) for block in blocks}
    while pending:
        reply = owner.read().get("Reply")
        if reply and reply["request"] in pending:
            assert "Ok" in reply["result"], reply
            pending.remove(reply["request"])
    owner.close()
    (host.root / "read").touch()
    receipt = host.root / "input.sha256"
    wait(lambda: receipt.exists())
    expected = hashlib.sha256("".join(blocks).encode()).hexdigest()
    assert receipt.read_text() == expected
    observer = host.wire()
    join(observer, session)
    wait(lambda: "INPUT-DONE" in text(observer.screen(pane)))
    observer.workspace({"Acquire": {"session": session, "fixed": None}})
    observer.workspace({"Close": {"Session": session}})
    emit("32768 admitted input bytes survive controller disconnect under PTY backpressure", expected)


def limits(host):
    owner = host.wire()
    accepted = []
    for _ in range(33):
        before = owner.catalog()
        messages = owner.workspace({"Create": geometry(2, 1)}, error="capacity") if len(accepted) == 32 else owner.workspace({"Create": geometry(2, 1)})
        if len(accepted) == 32:
            assert owner.catalog() == before
            break
        accepted.append(created(messages, "Session"))
    assert len(accepted) == 32
    for session in accepted:
        join(owner, session, geometry(2, 1))
        owner.workspace({"Acquire": {"session": session, "fixed": None}})
        owner.workspace({"Close": {"Session": session}})
    assert sessions(owner) == []
    emit("32-pane admission cap refuses atomically and recovers after close", True)


def run(host):
    output(host)
    input(host)
    limits(host)
