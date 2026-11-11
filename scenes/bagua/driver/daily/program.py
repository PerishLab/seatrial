import json
import os
import select
import signal
import termios
import time
import tty


def hold():
    while True:
        signal.pause()


def history(root):
    tty.setraw(0)
    width = termios.tcgetwinsize(0)[1]
    for index in range(50024):
        row = (f"{index:08d}:" + "x" * (width - 9)).encode() + b"\r\n"
        offset = 0
        while offset < len(row):
            offset += os.write(1, row[offset:])
    os.write(1, b"HISTORY-DONE")
    (root / "history.done").touch()
    while True:
        value = os.read(0, 1)
        os.write(1, replies()[value])
        (root / "history.ack").write_bytes(value)


def replies():
    return {b"a": b"\x1b[?1049h" + b"alternate\r\n" * 100 + b"\x1b[?1049l", b"c": b"\x1b[3J", b"o": b"EXTRA-ROW\r\n" * 100, b"r": b"\x1bc"}


def keyboard(root):
    tty.setraw(0)
    flags = int((root / "flags").read_text())
    os.write(1, f"\x1b[>{flags}u\x1b[?u\x1b[c".encode())
    query = bytearray()
    while len(query) < 12:
        query.extend(os.read(0, 12 - len(query)))
    (root / "query.bin").write_bytes(query)
    os.write(1, b"KEYBOARD-READY")
    content = bytearray()
    while len(content) < 8:
        content.extend(os.read(0, 8 - len(content)))
    (root / "keys.bin").write_bytes(content)
    os.write(1, b"\x1b[<uLEGACY-READY")
    (root / "literal.bin").write_bytes(os.read(0, 1))
    os.write(1, b"KEYBOARD-DONE")
    hold()


def clipboard(root):
    tty.setraw(0)
    content = (root / "clipboard.txt").read_bytes()
    os.write(1, b"\x1b]52;c;dW50cnVzdGVk\x1b\\\x1b[2J\x1b[H")
    offset = 0
    while offset < len(content):
        offset += os.write(1, content[offset:])
    (root / "clipboard.ready").touch()
    hold()


def counter(root):
    tty.setraw(0)
    for index in range(1, 61):
        os.write(1, f"\rCOUNTER{index:04d}".encode())
        time.sleep(0.02)
    os.write(1, b"\r\nCOMPLETE\r\n")
    hold()


def untracked(root):
    tty.setraw(0, termios.TCSANOW)
    os.write(1, b"\x1b[2J\x1b[HREADY")
    with (root / "plain.bin").open("ab", buffering=0) as output:
        while True:
            value = os.read(0, 4096)
            if not value:
                return
            output.write(value)
            os.write(1, value)


def protocol(root):
    tty.setraw(0)
    steps = [(b"\x1b[>5u", 1), (b"\x1b[>7u", 1), (b"\x1b[<2u", 0), (b"\x1b[>1u\x1b[?1049h", 0), (b"\x1b[>7u", 1), (b"\x1b[?1049l", 1), (b"\x1b[<u", 0), (b"\x1b[=31u", 1), (b"\x1b[=1;3u", 0), (b"\x1b[=65535u", 1), (b"\x1b[=2u", 0), (b"\x1b[>7u\x1bc", 0), (b"\x1b[>1;2u", 0)]
    results = []
    for sequence, flags in steps:
        os.write(1, sequence + b"\x1b[?u")
        reply = bytearray()
        while len(reply) < 5:
            reply.extend(os.read(0, 5 - len(reply)))
        expected = f"\x1b[?{flags}u".encode()
        assert reply == expected, (sequence, reply, expected)
        results.append({"request": sequence.hex(), "reply": reply.hex()})
    (root / "protocol.json").write_text(json.dumps(results))
    os.write(1, b"PROTOCOL-DONE")
    hold()


def departure(root):
    pid = os.getpid()
    settings = json.loads((root / "departure.json").read_text())
    tty.setraw(0)
    os.write(1, b"DEPARTURE-READY")
    if settings.get("eof"):
        for fd in (0, 1, 2):
            os.close(fd)
    else:
        data = b"TAIL" * 32768
        offset = 0
        while offset < len(data):
            offset += os.write(1, data[offset:])
    while not (root / ("release-" + str(pid))).exists():
        time.sleep(0.01)
    if settings.get("signal"):
        os.kill(pid, signal.SIGTERM)
    raise SystemExit(settings.get("code", 0))


def motion(root):
    tty.setraw(0)
    path = root / ("motion-" + str(os.getpid()))
    os.write(1, b"\x1b[2J\x1b[HMOTION-READY\x1b[?1006h")
    with path.with_suffix(".bin").open("ab", buffering=0) as output:
        monitor(path, output)


def monitor(path, output):
    previous = "off"
    while True:
        mode = path.with_suffix(".mode").read_text() if path.with_suffix(".mode").exists() else "off"
        if mode != previous:
            os.write(1, b"\x1b[?1003h" if mode == "on" else b"\x1b[?1003l")
            previous = mode
        if select.select([0], [], [], 0.01)[0]:
            value = os.read(0, 4096)
            if not value:
                return
            output.write(value)


def modes():
    return {"motion": motion, "departure": departure, "history": history, "keyboard": keyboard, "clipboard": clipboard, "counter": counter, "untracked": untracked, "protocol": protocol}
