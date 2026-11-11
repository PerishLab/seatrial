import json

from host import emit
from terminal import Terminal
from wire import wait
from workspace import join
from ..common import active, command, config, data, launch, profile, sized, state, until
from ..entry import describe


def row(view, index):
    view.drain()
    return "".join(view.view.cells[index])


def close(view, client, session):
    view.detach(b"\x07d")
    client.workspace({"Acquire": {"session": session, "fixed": None}})
    client.workspace({"Close": {"Session": session}})


def defaults(host):
    view = Terminal(host, launch(host, "echo", ["--no-config"]), (100, 24))
    client = host.wire()
    try:
        until(view, "READY")
        until(view, "W1:")
        value = state(client)
        join(client, value["id"])
        assert value["header"] and value["edge"] == "top"
        assert sized(host) == {"columns": 100, "rows": 21}
        wait(lambda: "1:" in row(view, 23))
        wait(lambda: row(view, 22).strip() == "")
        assert view.view.colors[0][0] == 25
        assert 39 in view.view.colors[23]
        assert not view.view.osc
        assert (view.view.x, view.view.y) == (5, 1)
        view.write(b"\x1b[<0;2;2M")
        wait(lambda: data(host, "input.bin").endswith(b"\x1b[<0;2;1M"))
        view.resize(20, 12)
        wait(lambda: sized(host) == {"columns": 20, "rows": 9})
        wait(lambda: "1:" in row(view, 11))
        assert ":" not in row(view, 11).rsplit("1:", 1)[-1]
        close(view, client, value["id"])
        emit("unconfigured templates reserve header and two footer rows, active colors and blank region; actual cursor/mouse and narrow navigation", True)
    finally:
        view.close()


def settings():
    return profile() + """
[header]
position='bottom'
border=false
format='H{{pane.index}}{{#if pane.command}} cmd{{/if}}'
[status]
position='top'
[[status.lines]]
left='L{{session.name:4}}'
center='C'
right='R{{client.columns}}'
[[status.lines]]
left='{{#if client.columns >= 64}}wide{{else}}compact{{/if}} {{#windows}}{{#if window.active}}{{style active}}{{else}}{{style inactive}}{{/if}}[{{window.index}}:{{window.name}}]{{style base}}{{/windows}}'
"""


def custom(host):
    original = settings()
    args = launch(host, "echo", config(host, original))
    index = args.index("open")
    args[index:index + 1] = ["enter", "{{clock.time}}"]
    view = Terminal(host, args, (100, 24))
    client = host.wire()
    observer = None
    try:
        until(view, "READY")
        wait(lambda: row(view, 0).endswith("R100"))
        assert row(view, 0).startswith("L{{cl") and view.view.cells[0][49] == "C"
        wait(lambda: "wide [1:" in row(view, 1))
        wait(lambda: "H1 cmd" in row(view, 23))
        assert (view.view.x, view.view.y) == (5, 2)
        assert sized(host) == {"columns": 100, "rows": 21}
        value = state(client)
        join(client, value["id"])
        pane = active(client)
        pid = describe(client, pane)["pid"]
        assert value["edge"] == "bottom"
        view.write(b"\x1b[<0;2;3M")
        wait(lambda: data(host, "input.bin").endswith(b"\x1b[<0;2;1M"))
        path = host.root / "client.toml"
        path.write_text(original.replace("H{{pane.index}}", "{{unknown}}"))
        view.write(b"\x11r")
        until(view, "Configuration unchanged")
        assert describe(client, pane)["pid"] == pid and state(client)["edge"] == "bottom"
        view.resize(60, 14)
        wait(lambda: sized(host) == {"columns": 60, "rows": 11})
        wait(lambda: "compact [1:" in row(view, 1))
        path.write_text(original.replace("position='top'", "position='bottom'"))
        view.write(b"\x11r")
        until(view, "Configuration reloaded")
        wait(lambda: "compact [1:" in row(view, 13))
        assert describe(client, pane)["pid"] == pid
        path.write_text(profile() + "[header]\nformat=''\n[status]\n[[status.lines]]\nleft=''\n[[status.lines]]\nleft='NAV'\n")
        view.write(b"\x11r")
        until(view, "Configuration reloaded")
        wait(lambda: "NAV" in row(view, 13))
        view.write(b"\x11")
        until(view, "prefix pending")
        assert "NAV" in row(view, 13)
        view.write(b"\x1b")
        observer = Terminal(host, [*config(host, path.read_text(), "empty.toml"), "connect", str(value["id"])], (120, 14))
        until(observer, "Observe")
        before = state(client)
        observer.write(b"\x11x")
        until(observer, "requires the session controller")
        assert state(client)["sizing"] == before["sizing"]
        assert state(client)["control"] == before["control"]
        observer.detach(b"\x1bq")
        view.write(b"\x11[")
        until(view, "Copy")
        view.write(b"\x1b")
        view.detach(b"\x1bq")
        client.workspace({"Acquire": {"session": value["id"], "fixed": None}})
        client.workspace({"Close": {"Session": value["id"]}})
        emit("custom alignment/list/style/width and positions preserve actual PTY/cursor/mouse; atomic reload preserves PID; empty templates cannot hide role/prefix/copy/refusal", True)
    finally:
        view.close()
        if observer is not None:
            observer.close()


def strict(host):
    failures = [
        "[header]\nformat='{{unknown}}'\n",
        "[header]\nformat='{{#if pane.active}}x'\n",
        "[header]\nformat='{{#windows}}{{#windows}}{{/windows}}{{/windows}}'\n",
        "[header]\nformat='{{pane.command:513}}'\n",
        "[header]\nformat='{{#if session.name >= 2}}x{{/if}}'\n",
        "[header]\nformat='" + "x" * 4097 + "'\n",
        "[header]\nformat='" + "{{pane.id}}" * 129 + "'\n",
        "[header]\nformat=\"\\u001b\"\n",
        "[header]\ncomponents=['pane']\nformat='x'\n",
        "[status]\ncomponents=['clock']\n[[status.lines]]\nleft='x'\n",
        "[status]\nlines=[]\n",
        "[[status.lines]]\nleft='x'\n" * 5,
    ]
    for text in failures:
        result = command(host, [*config(host, text, "invalid-bars.toml"), "config"])
        assert result.returncode != 0, text[:128]
    result = command(host, ["--no-config", "config"])
    value = json.loads(result.stdout)
    assert value["header"]["mode"] == "template" and value["status"]["rows"] == 2
    assert "client.columns" in value["status"]["lines"][1]["left"]
    emit("formal executable rejects unknown/malformed/recursive/oversize/control/mixed templates; compiled default report matches two-layer contract", len(failures))


def run(host):
    strict(host)
    defaults(host)
    custom(host)
