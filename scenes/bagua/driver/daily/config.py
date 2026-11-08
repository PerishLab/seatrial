import json

from host import emit
from terminal import Terminal
from wire import wait
from .common import command, config, launch, panes, profile, state, until


def report(host, args, env=None):
    result = command(host, [*args, "config"], env)
    assert result.returncode == 0, result.stderr
    return json.loads(result.stdout)


def discovery(host):
    home = host.home / ".config/bagua"
    home.mkdir(parents=True)
    path = home / "bagua.toml"
    path.write_text("[client]\nprefix='C-a'\n")
    assert report(host, [])["client"]["prefix"] == "C-a"
    xdg = host.root / "xdg"
    (xdg / "bagua").mkdir(parents=True)
    (xdg / "bagua/bagua.toml").write_text("[client]\nprefix='C-q'\n")
    env = dict(host.env, XDG_CONFIG_HOME=str(xdg))
    assert report(host, [], env)["client"]["prefix"] == "C-q"
    (host.root / "bagua.toml").write_text("version=999\n")
    assert report(host, ["--no-config"], env)["client"]["prefix"] == "C-g"
    settings = config(host, "[client]\nprefix='C-b'\n")
    assert report(host, settings, env)["client"]["prefix"] == "C-b"
    assert report(host, [], dict(host.env, XDG_CONFIG_HOME=str(host.root / "absent")))["client"]["prefix"] == "C-g"
    path.unlink()
    emit("explicit > XDG > HOME; no working-directory search; no-config bypass", True)


def strict(host):
    failures = ["version=2", "unknown=true", "[client]\nunknown=1", "[history]\nlines=50001", "[status]\ntimezone='local'", "[header]\ncomponents=['cwd','cwd']", "[clipboard]\nlimit=0", "[mouse]\nresize='yes'", "[bindings.prefix]\nx='unknown'", "[client]\nescape_ms=-1", "invalid = ["]
    for value in failures:
        result = command(host, [*config(host, value), "config"])
        assert result.returncode != 0, value
    contents = "[program]\nexecutable='/bin/sh'\nargs=['SCENE-PRIVATE-ARG']\ncwd='/'\nenvironment={TOKEN='SCENE-PRIVATE-VALUE'}\n"
    result = report(host, config(host, contents))
    assert result["program"]["args"] == 1
    assert result["program"]["environment"] == ["TOKEN"]
    assert "SCENE-PRIVATE" not in json.dumps(result)
    result = report(host, config(host, "[host]\nsocket='/$HOME/literal.sock'\n"))
    assert result["host"]["socket"] == "/$HOME/literal.sock"
    emit("strict unknown/type/range/action refusal and redacted program values; literal path", True)


def reload(host):
    original = profile()
    settings = config(host, original)
    view = Terminal(host, launch(host, "untracked", settings), (100, 25))
    client = host.wire()
    try:
        until(view, "READY")
        before = state(client)
        path = host.root / "client.toml"
        for text in [original.replace("C-q", "C-a") + "[status]\ntimezone='local'\n", original + "[history]\nlines=100\n"]:
            path.write_text(text)
            view.write(b"\x11r")
            until(view, "Configuration unchanged")
            view.write(b"\x11?")
            until(view, "Prefix:")
        path.write_text(original.replace("C-q", "C-a").replace("split-rows", "window") + "[mouse]\ncopy_on_release=false\n[clipboard]\ntarget='local'\n[header]\nenabled=true\n[status]\nseparator=true\n")
        view.write(b"\x11r")
        until(view, "Configuration reloaded")
        wait(lambda: state(client)["header"])
        view.write(b"\x01-")
        wait(lambda: len(state(client)["windows"]) == 2)
        after = state(client)
        assert before["id"] == after["id"] and before["control"] == after["control"]
        assert all(pane["state"] == "Running" for pane in panes(after))
        view.detach(b"\x1bq")
        client.workspace({"Join": {"session": after["id"], "viewport": {"columns": 100, "rows": 25}}})
        client.workspace({"Acquire": {"session": after["id"], "fixed": None}})
        client.workspace({"Close": {"Session": after["id"]}})
        emit("failed whole reload and startup changes refuse; successful keys/mouse/clipboard/decor reload preserves identity", True)
    finally:
        view.close()


def run(host):
    discovery(host)
    strict(host)
    reload(host)
