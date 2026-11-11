from pathlib import Path

from daily.common import legacy as config, panes, profile, until
from daily.entry import describe, finished, shell
from host import sessions
from terminal import Terminal
from wire import text, wait
from workspace import join


def template(host, mode):
    if mode == "shell":
        return shell(host)
    program = host.program("untracked")
    executable = program["executable"]
    if mode == "transient":
        path = host.root / "transient-python"
        path.symlink_to(executable)
        executable = str(path)
    args = ["--executable", executable, "--cwd", program["cwd"]]
    args.extend("--arg=" + arg for arg in program["args"])
    for name, value in program["environment"].items():
        args.extend(["--env", name + "=" + value])
    return args


class Trial:
    def __init__(self, host, name, mode="untracked"):
        self.host = host
        self.name = name
        self.settings = config(host, profile(), name + ".toml")
        self.views = []
        self.view = self.terminal([*self.settings, "enter", name, *template(host, mode)])
        until(self.view, "control")
        if mode != "shell":
            until(self.view, "READY")
        self.client = host.wire()
        value = next(value for value in sessions(self.client) if value["name"] == name)
        self.session = value["id"]
        join(self.client, self.session)
        self.pid = describe(self.client, self.pane())["pid"]
        host.children.append(self.pid)

    def terminal(self, args, size=(120, 28)):
        view = Terminal.launch(self.host, args, size)
        self.views.append(view)
        return view

    def state(self):
        return next(value for value in sessions(self.client) if value["id"] == self.session)

    def pane(self):
        value = self.state()
        return value["windows"][str(value["selected"])]["focus"]

    def owner(self):
        return self.state()["control"]["owner"]

    def bodies(self):
        return {pane["id"]: text(self.client.screen(pane["id"])) for pane in panes(self.state())}

    def close(self):
        owned = [describe(self.client, pane["id"])["pid"] for pane in panes(self.state()) if pane["state"] == "Running"]
        self.host.children.extend(owned)
        for view in self.views:
            view.close()
        wait(lambda: self.owner() is None)
        self.client.workspace({"Acquire": {"session": self.session, "fixed": None}})
        self.client.workspace({"Close": {"Session": self.session}})
        self.client.close()
        assert all(not Path(f"/proc/{pid}").exists() for pid in owned)
