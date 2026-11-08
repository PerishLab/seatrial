import json
import subprocess
import time

from host import sessions
from wire import wait


def command(host, args, env=None):
    return subprocess.run([host.binary, *args], env=env or host.env, cwd=host.root, capture_output=True, text=True, timeout=15)


def config(host, contents, name="client.toml"):
    path = host.root / name
    path.write_text(contents)
    return ["--config", str(path)]


def profile():
    return """version=1
[client]
prefix='C-q'
escape_ms=50
[bindings.prefix]
'\\'='split-columns'
'-'='split-rows'
'='='window'
r='reload'
[bindings.direct]
Alt-q='detach'
"""


def launch(host, mode, settings=None):
    program = host.program(mode)
    args = (settings or ["--no-config"]) + ["open", "--executable", program["executable"], "--cwd", program["cwd"]]
    args.extend("--arg=" + arg for arg in program["args"])
    for name, value in program["environment"].items():
        args.extend(["--env", name + "=" + value])
    return args


def state(client):
    return sessions(client)[-1]


def panes(value):
    return [pane for window in value["windows"].values() for pane in window["panes"].values()]


def active(client):
    value = state(client)
    return value["windows"][str(value["selected"])]["focus"]


def until(view, value):
    try:
        wait(lambda: value in view.screen())
    except AssertionError:
        print(json.dumps({"wanted": value, "screen": view.screen()[-16384:], "client": view.process.poll(), "bytes": view.total}), flush=True)
        raise


def settle(view, seconds=0.15):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        view.drain()
        time.sleep(0.01)


def data(host, name):
    path = host.root / name
    return path.read_bytes() if path.exists() else b""


def sized(host, name="echo.size"):
    value = data(host, name)
    return json.loads(value) if value else None


def send(client, pane, value):
    return client.ask({"Send": {"pane": pane, "input": {"Text": value}}})
