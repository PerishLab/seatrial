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
from .view import View


class Terminal:
    def __init__(self, host, args, size=(80, 25)):
        self.begin(host,args,size)
        wait(lambda: self.entered())
        assert self.process.poll() is None, bytes(self.output)

    @classmethod
    def launch(cls, host, args, size=(80,25)):
        view=cls.__new__(cls)
        view.begin(host,args,size)
        return view

    def begin(self,host,args,size):
        self.master, self.slave = os.openpty()
        self.before = termios.tcgetattr(self.slave)
        self.output = bytearray()
        self.total = 0
        self.view=View(*size)
        self.resize(*size)
        command=[host.binary]
        if host.path is not None:
            command.extend(["--socket",str(host.path)])
        command.extend(args)
        self.process=subprocess.Popen(command,env=host.env,stdin=self.slave,stdout=self.slave,stderr=self.slave,preexec_fn=self.claim)
        os.set_blocking(self.master,False)

    def claim(self):
        os.setsid()
        fcntl.ioctl(self.slave, termios.TIOCSCTTY, 0)

    def resize(self, columns, rows):
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0))
        self.view.resize(columns, rows)

    def drain(self):
        for _ in range(32):
            if not select.select([self.master], [], [], 0)[0]:
                break
            try:
                data = os.read(self.master, 65536)
            except BlockingIOError:
                break
            self.total += len(data)
            self.view.feed(data)
            self.output.extend(data)
            if len(self.output) > 2097152:
                del self.output[:-1048576]
        return bytes(self.output)

    def entered(self):
        return b"\x1b[?1049h" in self.drain() or self.process.poll() is not None

    def visible(self):
        return re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", self.drain())

    def screen(self):
        self.drain()
        return self.view.text()

    def write(self, data):
        deadline=time.monotonic()+10
        while data:
            assert time.monotonic()<deadline, "terminal input admission timeout"
            try:
                count=os.write(self.master,data)
                assert count>0
                data=data[count:]
            except BlockingIOError:
                self.drain()
                time.sleep(0.002)

    def detach(self, keys=b"\x07d"):
        self.write(keys)
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
