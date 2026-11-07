import fcntl
import os
import re
import select
import signal
import struct
import subprocess
import termios
import time

from wire import wait


class Terminal:
    def __init__(self, host, args, size=(80, 25)):
        self.master, self.slave = os.openpty()
        self.before = termios.tcgetattr(self.slave)
        self.output = bytearray()
        self.resize(*size)
        command = [host.binary, "--socket", str(host.path), *args]
        self.process = subprocess.Popen(command, env=host.env, stdin=self.slave, stdout=self.slave, stderr=self.slave, preexec_fn=self.claim)
        wait(lambda: self.entered())
        assert self.process.poll() is None, bytes(self.output)

    def claim(self):
        os.setsid()
        fcntl.ioctl(self.slave, termios.TIOCSCTTY, 0)

    def resize(self, columns, rows):
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0))

    def drain(self):
        for _ in range(32):
            if not select.select([self.master], [], [], 0)[0]:
                break
            data = os.read(self.master, 65536)
            self.output.extend(data)
            if len(self.output) > 2097152:
                del self.output[:-1048576]
        return bytes(self.output)

    def entered(self):
        return b"\x1b[?1049h" in self.drain() or self.process.poll() is not None

    def visible(self):
        return re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", self.drain())

    def write(self, data):
        assert os.write(self.master, data) == len(data)

    def detach(self):
        self.write(b"\x07d")
        deadline = time.monotonic() + 10
        while self.process.poll() is None and time.monotonic() < deadline:
            self.drain()
            time.sleep(0.01)
        assert self.process.wait(timeout=1) == 0, self.drain()
        assert b"\x1b[?1049l" in self.drain()
        assert termios.tcgetattr(self.slave) == self.before

    def close(self):
        if self.process.poll() is None:
            self.process.send_signal(signal.SIGKILL)
            self.process.wait(timeout=5)
        os.close(self.master)
        os.close(self.slave)
