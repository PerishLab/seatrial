import hashlib
import json
import os
from pathlib import Path
import select
import signal
import sys
import termios
import time
import tty


def record(root, name, value):
    temporary = root / (name + "." + str(os.getpid()) + ".next")
    temporary.write_text(value)
    temporary.replace(root / name)


def echo(root):
    tty.setraw(0)
    pending = [False]
    def resized(signum, frame):
        pending[0] = True
    def size():
        rows, columns = termios.tcgetwinsize(0)
        record(root, "echo.size", json.dumps({"columns": columns, "rows": rows}))
    signal.signal(signal.SIGWINCH, resized)
    size()
    os.write(1, b"\x1b]52;c;bagua-outer-secret\x07\x1b[2J\x1b[HREADY\x1b[?2004h\x1b[?1000h\x1b[?1006h")
    with (root / "input.bin").open("ab", buffering=0) as log:
        while True:
            if pending[0]:
                pending[0] = False
                size()
            if not select.select([0], [], [], 0.05)[0]:
                continue
            data = os.read(0, 4096)
            if not data:
                return
            log.write(data)
            os.write(1, data)


def flood(root):
    tty.setraw(0)
    data = b"X\x07" * 524288
    count = 0
    while count < len(data):
        count += os.write(1, data[count:count + 4096])
    (root / "output.json").write_text(str(count))
    for fragment in [b"\xe4", b"\xbd", b"\xa0", b"\xe5\xa5", b"\xbd"]:
        os.write(1, fragment)
        time.sleep(0.02)
    os.write(1, b"FLOW-DONE")
    time.sleep(120)


def slow(root):
    tty.setraw(0)
    (root / "ready").touch()
    while not (root / "read").exists():
        time.sleep(0.01)
    data = bytearray()
    while len(data) < 32768:
        data.extend(os.read(0, min(1024, 32768 - len(data))))
        time.sleep(0.003)
    record(root, "input.sha256", hashlib.sha256(data).hexdigest())
    os.write(1, b"INPUT-DONE")
    time.sleep(120)


def exited(root):
    os.write(1, b"EXIT-DONE")
    sys.exit(23)


def eof(root):
    for fd in (0, 1, 2):
        os.close(fd)
    time.sleep(120)


def stubborn(root):
    signal.signal(signal.SIGHUP, signal.SIG_IGN)
    (root / "ready").touch()
    time.sleep(120)


def main():
    mode = sys.argv[1]
    root = Path(sys.argv[2])
    (root / (mode + ".pid")).write_text(str(os.getpid()))
    (root / ("child-" + str(os.getpid()) + ".pid")).write_text(str(os.getpid()))
    modes = {"echo": echo, "flood": flood, "slow": slow, "exit": exited, "eof": eof, "stubborn": stubborn}
    modes[mode](root)


if __name__ == "__main__":
    main()
