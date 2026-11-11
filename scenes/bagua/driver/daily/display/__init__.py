import datetime
import re
import time

from host import emit
from terminal import Terminal
from wire import geometry, wait
from workspace import join
from ..common import active, legacy as config, launch, panes, profile, settle, state, until
from ..entry import shell
from . import bars


def settings():
    return profile() + "[header]\nenabled=true\n[status]\nseparator=true\ntimezone='Asia/Shanghai'\nrefresh_seconds=1\n"


def budget(host):
    original = settings()
    view = Terminal(host, [*config(host, original), "open", *shell(host)], (100, 24))
    client = host.wire()
    observer = None
    try:
        until(view, "cwd~")
        value = state(client)
        join(client, value["id"])
        wait(lambda: state(client)["header"])
        pane = active(client)
        assert state(client)["sizing"] == {"Automatic": geometry(100, 22)}
        assert client.screen(pane)["extent"] == {"width": 100, "height": 21}
        view.write(b"printf 'BUDGET-'; stty size\r")
        until(view, "BUDGET-21 100")
        directory = host.root / "display-cwd"
        directory.mkdir()
        view.write(f"cd '{directory}'\r".encode())
        until(view, "cwd~" + str(directory))
        assert view.view.colors[0][0] == 4
        observer = Terminal(host, [*config(host, "", "observer-legacy.toml"), "connect", str(value["id"])], (100, 24))
        settle(observer)
        assert client.screen(pane)["extent"]["height"] == 21
        observer.write(b"\x07h")
        settle(observer)
        assert state(client)["header"]
        observer.detach()
        view.write(b"\x11")
        until(view, "prefix pending")
        view.write(b"?")
        until(view, "Prefix:")
        path = host.root / "client.toml"
        config(host, original.replace("Asia/Shanghai", "local").replace("C-q", "C-a"))
        view.write(b"\x11r")
        until(view, "Configuration unchanged")
        config(host, original.replace("enabled=true", "enabled=false").replace("separator=true", "separator=false").replace("C-q", "C-a"))
        view.write(b"\x11r")
        until(view, "Configuration reloaded")
        wait(lambda: not state(client)["header"])
        wait(lambda: client.screen(pane)["extent"]["height"] == 23)
        view.write(b"printf 'FULL-'; stty size\r")
        until(view, "FULL-23 100")
        view.detach(b"\x1bq")
        client.workspace({"Acquire": {"session": value["id"], "fixed": None}})
        client.workspace({"Close": {"Session": value["id"]}})
        emit("header/separator reserve actual application rows; foreground cwd is observed; observer cannot reserve; reload is atomic", True)
    finally:
        view.close()
        if observer is not None:
            observer.close()


def tiny(host):
    view = Terminal(host, [*config(host, settings()), "open", *shell(host)], (160, 3))
    client = host.wire()
    try:
        until(view, "Header reservation unavailable")
        value = state(client)
        assert not value["header"]
        assert value["sizing"] == {"Automatic": geometry(160, 1)}
        view.write(b"printf 'TINY-%s' \"$(stty size)\"\r")
        until(view, "TINY-1 160")
        view.detach(b"\x1bq")
        join(client, value["id"])
        client.workspace({"Acquire": {"session": value["id"], "fixed": None}})
        client.workspace({"Close": {"Session": value["id"]}})
        emit("narrow feedback has priority; failed header reservation leaves live application input usable", True)
    finally:
        view.close()


def traffic(host):
    decor = profile() + "[status]\ntimezone='Asia/Shanghai'\nrefresh_seconds=1\n"
    view = Terminal(host, launch(host, "counter", config(host, decor)), (80, 24))
    client = host.wire()
    try:
        time.sleep(0.4)
        until(view, "COMPLETE")
        assert "COUNTER0060" in view.screen()
        assert view.total < 100000, view.total
        before = view.total
        settle(view, 2.2)
        assert view.total - before < 3000, view.total - before
        clocks = {((datetime.datetime.now(datetime.timezone.utc) + datetime.timedelta(hours=8, minutes=offset)).strftime("%H:%M")) for offset in (-1, 0, 1)}
        assert any(clock in view.screen().splitlines()[-1] for clock in clocks), view.screen()
        value = state(client)
        view.detach(b"\x1bq")
        join(client, value["id"])
        client.workspace({"Acquire": {"session": value["id"], "fixed": None}})
        client.workspace({"Close": {"Session": value["id"]}})
        emit("slow outer reader sees latest complete UTF-8 rows; counter update traffic bounded; idle clock limited to status row", {"bytes": view.total, "idle": view.total - before})
    finally:
        view.close()


def run(host):
    bars.run(host)
    budget(host)
    tiny(host)
    traffic(host)
