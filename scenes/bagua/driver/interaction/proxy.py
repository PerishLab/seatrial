import json
import os
import select
import socket
import struct
import threading


def packets(buffer, limit):
    while len(buffer) >= 4:
        size = struct.unpack("!I", buffer[:4])[0]
        assert 0 < size <= limit
        if len(buffer) < size + 4:
            return
        raw = bytes(buffer[:size + 4])
        value = json.loads(raw[4:])
        del buffer[:size + 4]
        yield value, raw


class Proxy:
    def __init__(self, host, mode):
        self.path = host.root / ("proxy-" + mode + ".sock")
        assert not self.path.exists()
        self.listener = socket.socket(socket.AF_UNIX)
        self.listener.bind(str(self.path))
        self.inode = self.path.stat().st_ino
        self.path.chmod(0o600)
        self.listener.listen(1)
        self.listener.settimeout(8)
        self.mode = mode
        self.target = None
        self.sync = None
        self.pending = []
        self.peers = []
        self.fault = None
        self.observed = threading.Event()
        self.held = threading.Event()
        self.released = threading.Event()
        self.stopped = threading.Event()
        self.thread = threading.Thread(target=self.run, args=(host.path,))
        self.thread.start()

    def request(self, value):
        command = value["command"].get("Workspace") if isinstance(value["command"], dict) else None
        target = "Split" if self.mode == "refuse" else "Focus"
        if self.target is None and isinstance(command, dict) and target in command:
            self.target = value["id"]
            self.observed.set()
        if self.target is not None and command == "Read" and self.mode == "recover":
            assert self.sync is None
            self.sync = value["id"]

    def response(self, value, raw):
        reply = value.get("Reply", {})
        finished = value.get("Finished", {})
        if self.target is not None and self.mode == "refuse" and reply.get("request") == self.target:
            assert "Err" in reply["result"], value
            self.pending.append(raw)
            self.held.set()
            return False
        if self.target is not None and self.mode == "recover" and reply.get("request") == self.target:
            assert "Changed" in reply["result"]["Ok"]["Applied"]["Workspace"]
            return False
        request = reply.get("request", finished.get("request"))
        if self.sync is not None and request == self.sync:
            self.pending.append(raw)
            self.held.set()
            return False
        return True

    def run(self, path):
        try:
            downstream, _ = self.listener.accept()
            self.peers = [downstream]
            upstream = socket.socket(socket.AF_UNIX)
            self.peers.append(upstream)
            upstream.settimeout(8)
            upstream.connect(str(path))
            self.peers = [downstream, upstream]
            downstream.settimeout(8)
            self.forward(downstream, upstream)
        except Exception as error:
            if not self.stopped.is_set():
                self.fault = error
        finally:
            for peer in self.peers:
                peer.close()

    def forward(self, downstream, upstream):
        buffers = {downstream: bytearray(), upstream: bytearray()}
        while not self.stopped.is_set():
            if self.released.is_set() and self.pending:
                for raw in self.pending:
                    downstream.sendall(raw)
                self.pending.clear()
            readable, _, _ = select.select(self.peers, [], [], 0.02)
            for peer in readable:
                data = peer.recv(65536)
                if not data:
                    return
                buffers[peer].extend(data)
                self.frames(peer, buffers[peer])

    def frames(self, peer, buffer):
        downstream, upstream = self.peers
        limit = 131072 if peer is downstream else 8388608
        for value, raw in packets(buffer, limit):
            if peer is downstream:
                self.request(value)
                upstream.sendall(raw)
            elif self.response(value, raw):
                downstream.sendall(raw)

    def resume(self):
        self.released.set()

    def close(self):
        self.stopped.set()
        self.listener.close()
        for peer in self.peers:
            try:
                peer.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
        self.thread.join(timeout=10)
        assert not self.thread.is_alive()
        assert self.fault is None, repr(self.fault)
        assert self.path.stat().st_ino == self.inode
        self.path.unlink()
