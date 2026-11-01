import select
import socket
import socketserver
import threading


def relay(left, right):
    while True:
        ready, _, _ = select.select([left, right], [], [], 5)
        if not ready:
            return
        for source in ready:
            data = source.recv(65536)
            if not data:
                return
            target = right if source is left else left
            target.sendall(data)


class Handler(socketserver.StreamRequestHandler):
    def handle(self):
        request = self.rfile.readline(4096).decode().split()
        assert request == ["CONNECT", "releases.concord.perish.uk:443", "HTTP/1.1"], "foreign proxy target"
        for _ in range(64):
            line = self.rfile.readline(4096)
            if line in [b"\r\n", b"\n"]:
                break
        else:
            raise AssertionError("proxy headers exceed bounds")
        with self.server.lock:
            self.server.count += 1
            number = self.server.count
        if number in self.server.failures:
            status = 503 if number == 1 else 502
            self.wfile.write(f"HTTP/1.1 {status} SCENE_SECRET\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".encode())
            return
        with socket.create_connection(("releases.concord.perish.uk", 443), timeout=2) as upstream:
            self.wfile.write(b"HTTP/1.1 200 Connection established\r\n\r\n")
            self.wfile.flush()
            relay(self.connection, upstream)


class Proxy:
    def __init__(self, failures):
        self.server = socketserver.ThreadingTCPServer(("127.0.0.1", 0), Handler)
        self.server.failures = failures
        self.server.count = 0
        self.server.lock = threading.Lock()
        self.thread = threading.Thread(target=self.server.serve_forever)

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, kind, value, trace):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=10)
        assert not self.thread.is_alive(), "proxy cleanup unknown"

    def address(self):
        return f"http://127.0.0.1:{self.server.server_address[1]}"
