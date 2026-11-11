from host import emit
from terminal import Terminal
from wire import wait
from workspace import join
from ..common import active, legacy as config, launch, profile, state, until


def reporting(view, mode):
    view.drain()
    return view.view.modes.get(mode, False)


def tracking(host):
    original = profile() + "[mouse]\ncopy_on_release=false\n"
    settings = config(host, original)
    view = Terminal(host, launch(host, "motion", settings), (80, 25))
    client = host.wire()
    observer = None
    try:
        until(view, "MOTION-READY")
        value = state(client)
        first = active(client)
        join(client, value["id"])
        path = host.root / ("motion-" + str(host.pid("motion")))
        wait(lambda: reporting(view, 1002))
        assert b"\x1b[?1003h" not in view.output
        path.with_suffix(".mode").write_text("on")
        wait(lambda: client.screen(first)["modes"]["tracking"] == "Motion")
        wait(lambda: reporting(view, 1003))
        view.write(b"\x1b[<35;3;2M")
        wait(lambda: path.with_suffix(".bin").read_bytes() == b"\x1b[<35;3;2M")
        path.with_suffix(".mode").write_text("off")
        wait(lambda: not reporting(view, 1003) and reporting(view, 1002))
        path.with_suffix(".mode").write_text("on")
        wait(lambda: reporting(view, 1003))
        view.write(b"\x11=")
        wait(lambda: len(state(client)["windows"]) == 2)
        wait(lambda: active(client) != first and client.screen(active(client))["modes"]["tracking"] == "Off")
        wait(lambda: not reporting(view, 1003) and reporting(view, 1002))
        assert client.screen(first)["modes"]["tracking"] == "Motion"
        view.write(b"\x1b1")
        wait(lambda: active(client) == first)
        wait(lambda: reporting(view, 1003))
        observer = Terminal(host, [*settings, "connect", str(value["id"])], (80, 25))
        wait(lambda: reporting(observer, 1002))
        assert b"\x1b[?1003h" not in observer.output
        observer.detach(b"\x1bq")
        view.write(b"\x1b[<4;1;1M\x1b[<36;5;1M\x1b[<4;5;1m")
        wait(lambda: "Copy" in view.screen())
        wait(lambda: not reporting(view, 1003) and reporting(view, 1002))
        view.write(b"\x1b")
        wait(lambda: reporting(view, 1003))
        config(host, original.replace("[mouse]\n", "[mouse]\nenabled=false\n"))
        view.write(b"\x11r")
        until(view, "Configuration reloaded")
        wait(lambda: all(not reporting(view, mode) for mode in [1000, 1002, 1003, 1006]))
        config(host, original)
        view.write(b"\x11r")
        wait(lambda: reporting(view, 1003))
        view.detach(b"\x1bq")
        assert all(not reporting(view, mode) for mode in [1000, 1002, 1003, 1006])
        client.workspace({"Acquire": {"session": value["id"], "fixed": None}})
        client.workspace({"Close": {"Session": value["id"]}})
        emit("outer mouse demand: idle drag, live Motion delivery, mode-only toggle, hidden windows, observer, frozen copy, reload and restoration", True)
    finally:
        view.close()
        if observer is not None:
            observer.close()
