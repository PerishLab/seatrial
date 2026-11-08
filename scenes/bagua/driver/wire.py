import json
import socket
import struct
import time


class Wire:
    def __init__(self, path):
        self.socket = socket.socket(socket.AF_UNIX)
        self.socket.settimeout(8)
        self.socket.connect(str(path))
        self.bytes = bytearray()
        self.next = 0
        self.hello = self.read()["Hello"]
        self.version = self.hello["version"]
        assert self.version == 7, self.hello

    def close(self):
        self.socket.close()

    def read(self):
        while len(self.bytes) < 4:
            self.receive()
        size = struct.unpack("!I", self.bytes[:4])[0]
        assert 0 < size <= 8388608
        while len(self.bytes) < size + 4:
            self.receive()
        value = json.loads(self.bytes[4:size + 4])
        del self.bytes[:size + 4]
        return value

    def receive(self):
        data = self.socket.recv(65536)
        assert data, "host closed connection"
        self.bytes.extend(data)

    def send(self, command):
        self.next += 1
        data = json.dumps({"version": self.version, "id": self.next, "command": command}).encode()
        assert len(data) <= 131072
        self.socket.sendall(struct.pack("!I", len(data)) + data)
        return self.next

    def ask(self, command, error=None):
        request = self.send(command)
        messages = []
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            message = self.read()
            messages.append(message)
            reply = message.get("Reply", {})
            if reply.get("request") == request and "Err" in reply.get("result", {}):
                failure = reply["result"]["Err"]
                code = failure["Workspace"] if isinstance(failure, dict) else failure
                assert code.lower() == error, messages
            finished = message.get("Finished", {})
            if finished.get("request") == request:
                self.check(messages, error)
                return messages
        raise AssertionError("request did not finish")

    def check(self, messages, error):
        result = messages[-1]["Finished"]["result"]
        if error is None:
            assert result == {"Ok": None}, messages
        else:
            assert result.get("Err", {}).get("code") == error, messages

    def workspace(self, operation, error=None):
        return self.ask({"Workspace": operation}, error)

    def catalog(self):
        messages = self.workspace("Read")
        replies = [m["Reply"] for m in messages if "Reply" in m]
        return replies[-1]["result"]["Ok"]["Applied"]["Workspace"]["Snapshot"]

    def inspect(self, pane):
        return self.ask({"Inspect": pane})

    def screen(self, pane):
        messages = self.inspect(pane)
        return next(m["Screen"]["state"] for m in messages if "Screen" in m)

    def process(self, pane):
        return next(m["Process"] for m in self.inspect(pane) if "Process" in m)

    def view(self, pane):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            message = self.read().get("Screen")
            if message and message["pane"] == pane:
                return message["state"]
        raise AssertionError("subscription did not present its target")


def created(messages, kind):
    for message in messages:
        reply = message.get("Reply", {}).get("result", {}).get("Ok", {})
        change = reply.get("Applied", {}).get("Workspace", {}).get("Changed", {})
        for target in change.get("created", []):
            if kind in target:
                return target[kind]
    raise AssertionError(f"no created {kind}: {messages}")


def geometry(columns=80, rows=24):
    return {"columns": columns, "rows": rows}


def wait(check):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        result = check()
        if result:
            return result
        time.sleep(0.02)
    raise AssertionError("condition did not become true")


def text(screen):
    return "".join(cell["text"] or " " for cell in screen["cells"] if cell["width"] != 0)
