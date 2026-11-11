from host import emit
from terminal import Terminal
from wire import text, wait
from workspace import join
from .common import active, legacy as config, data, launch, panes, profile, settle, state, until


def negotiated(host, flags):
    for name in ["query.bin", "keys.bin", "literal.bin"]:
        (host.root / name).unlink(missing_ok=True)
    (host.root / "flags").write_text(str(flags))
    settings = config(host, profile())
    view = Terminal(host, launch(host, "keyboard", settings), (120, 28))
    client = host.wire()
    try:
        until(view, "KEYBOARD-READY")
        assert data(host, "query.bin") == b"\x1b[?1u\x1b[?1;2c"
        value = state(client)
        pane = active(client)
        join(client, value["id"])
        assert client.screen(pane)["unsupported"] >= int(flags != 1)
        view.write(b"\r\x1b[13;2u")
        wait(lambda: data(host, "keys.bin") == b"\r\x1b[13;2u")
        until(view, "LEGACY-READY")
        view.write(b"\x11\x11")
        wait(lambda: data(host, "literal.bin") == b"\x11")
        until(view, "KEYBOARD-DONE")
        view.detach(b"\x1bq")
        assert b"\x1b[<u" in view.output
        client.workspace({"Acquire": {"session": value["id"], "fixed": None}})
        client.workspace({"Close": {"Session": value["id"]}})
        emit("consumer compound keyboard request preserves actual flag1, distinct Enter/ShiftEnter and prefix literal after pop", flags)
    finally:
        view.close()


def routing(host):
    settings = config(host, profile())
    view = Terminal(host, launch(host, "untracked", settings), (100, 25))
    client = host.wire()
    try:
        until(view, "READY")
        initial = state(client)
        join(client, initial["id"])
        first = active(client)
        view.write(b"\x11\\BATCHED-SPLIT")
        wait(lambda: len(panes(state(client))) == 2)
        second = active(client)
        wait(lambda: "BATCHED-SPLIT" in text(client.screen(second)))
        assert "BATCHED-SPLIT" not in text(client.screen(first))
        view.write(b"\x11=WINDOW-ONE\x11=WINDOW-TWO\x1b1RETURN-FIRST")
        wait(lambda: len(state(client)["windows"]) == 3)
        wait(lambda: state(client)["selected"] == initial["selected"])
        wait(lambda: "RETURN-FIRST" in text(client.screen(second)))
        targets = {pane["id"]: text(client.screen(pane["id"])) for pane in panes(state(client))}
        assert sum("WINDOW-ONE" in body for body in targets.values()) == 1
        assert sum("WINDOW-TWO" in body for body in targets.values()) == 1
        assert all("RETURN-FIRST" not in body for pane, body in targets.items() if pane != second)
        view.write(b"\x11\x1b[200~=LEGAL-PASTE\x1b[201~")
        wait(lambda: data(host, "plain.bin").endswith(b"=LEGAL-PASTE"))
        until(view, "LEGAL-PASTE")
        assert len(state(client)["windows"]) == 3
        admitted = data(host, "plain.bin")
        view.write(b"\x1b[200~\x11=REJECTED-PASTE\x1b[201~")
        until(view, "payload exceeds")
        assert data(host, "plain.bin") == admitted
        assert all("REJECTED-PASTE" not in text(client.screen(pane["id"])) for pane in panes(state(client)))
        assert len(state(client)["windows"]) == 3
        before = text(client.screen(second))
        view.write(b"\x1b[97;1:3u")
        settle(view)
        assert text(client.screen(second)) == before
        until(view, "RETURN-FIRST")
        view.write(b"\x1b[<0;2;2MCLICK-FIRST")
        wait(lambda: active(client) == first)
        wait(lambda: "CLICK-FIRST" in text(client.screen(first)))
        assert "CLICK-FIRST" not in text(client.screen(second))
        view.write(b"\x11xAFTER-CLOSE")
        wait(lambda: len(panes(state(client))) == 3)
        wait(lambda: "AFTER-CLOSE" in text(client.screen(second)))
        view.detach(b"\x1bq")
        client.workspace({"Acquire": {"session": initial["id"], "fixed": None}})
        client.workspace({"Close": {"Session": initial["id"]}})
        emit("batched split/window/Alt-index/click/close keep trailing input with intended pane; paste and releases do not trigger layout", True)
    finally:
        view.close()


def stacks(host):
    view = Terminal(host, launch(host, "protocol"), (120, 28))
    client = host.wire()
    try:
        until(view, "PROTOCOL-DONE")
        import json
        results = json.loads(data(host, "protocol.json"))
        assert len(results) == 13
        value = state(client)
        view.detach()
        join(client, value["id"])
        client.workspace({"Acquire": {"session": value["id"], "fixed": None}})
        client.workspace({"Close": {"Session": value["id"]}})
        emit("keyboard stack/pop-count/set/clear, partial unknown masks, primary/alternate isolation, RIS and malformed push", results)
    finally:
        view.close()


def refusal(host):
    view = Terminal(host, launch(host, "untracked", config(host, profile())), (120, 25))
    client = host.wire()
    try:
        until(view, "READY")
        value = state(client)
        join(client, value["id"])
        view.write(b"\x11=" * 31)
        wait(lambda: len(state(client)["windows"]) == 32)
        wait(lambda: all(pane["state"] == "Running" for pane in panes(state(client))))
        view.write(b"\x11=REFUSED-NO-REPLAY")
        until(view, "Create window:")
        assert len(state(client)["windows"]) == 32
        assert all("REFUSED-NO-REPLAY" not in text(client.screen(pane["id"])) for pane in panes(state(client)))
        view.write(b"FRESH-AFTER-REFUSAL")
        wait(lambda: "FRESH-AFTER-REFUSAL" in text(client.screen(active(client))))
        assert all("REFUSED-NO-REPLAY" not in text(client.screen(pane["id"])) for pane in panes(state(client)))
        until(view, "w32")
        view.write(b"\x1b9")
        wait(lambda: state(client)["selected"] == list(state(client)["windows"].values())[8]["id"])
        until(view, "w9")
        view.detach(b"\x1bq")
        client.workspace({"Acquire": {"session": value["id"], "fixed": None}})
        client.workspace({"Close": {"Session": value["id"]}})
        emit("32 windows keep visible current 1-based index; capacity failure drops dependent input; fresh input resumes without Escape or replay", True)
    finally:
        view.close()


def run(host):
    for flags in (1, 5, 7):
        negotiated(host, flags)
    stacks(host)
    routing(host)
    refusal(host)
