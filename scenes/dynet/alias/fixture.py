import ipaddress
import socket
import socketserver
import struct
import sys
import threading


def encode(name):
    return b"".join(bytes([len(label)]) + label.encode("ascii") for label in name.split(".")) + b"\0"


def question(packet):
    start = 12
    labels = []
    while packet[start]:
        end = start + packet[start] + 1
        labels.append(packet[start + 1:end].decode("ascii"))
        start = end
    kind, family = struct.unpack_from("!HH", packet, start + 1)
    assert family == 1
    return ".".join(labels), kind, start + 5


def record(name, kind, data):
    life = 2 if name.startswith("short.") else 90
    return encode(name) + struct.pack("!HHIH", kind, 1, life, len(data)) + data


def answer(packet, ipv6):
    name, kind, end = question(packet)
    records = []
    part = name.split(".")[0]
    if name.endswith((".managed.test", ".direct.test", ".other.test")):
        prefix = part.split("-")[0]
        target = prefix + ".edge.test"
        if kind in (1, 28):
            records.append(record(name, 5, encode(target)))
            name = target
    data = {1: socket.inet_aton("203.0.113.7"), 28: ipaddress.IPv6Address(ipv6).packed}
    if kind in data and not (part == "partial" and records):
        records.append(record(name, kind, data[kind]))
    return packet[:2] + struct.pack("!HHHHH", 0x8180, 1, len(records), 0, 0) + packet[12:end] + b"".join(records)


class Datagram(socketserver.BaseRequestHandler):
    def handle(self):
        packet, sock = self.request
        sock.sendto(answer(packet, self.server.ipv6), self.client_address)


def read(sock, size):
    data = bytearray()
    while len(data) < size:
        part = sock.recv(size - len(data))
        if not part:
            raise EOFError()
        data.extend(part)
    return bytes(data)


class Stream(socketserver.BaseRequestHandler):
    def handle(self):
        size, = struct.unpack("!H", read(self.request, 2))
        reply = answer(read(self.request, size), self.server.ipv6)
        self.request.sendall(struct.pack("!H", len(reply)) + reply)


class Echo(socketserver.BaseRequestHandler):
    def handle(self):
        self.request.sendall(self.client_address[0].encode("ascii") + b"\n")


def echo(address):
    with socket.create_server((address, 8080), family=socket.AF_INET6) as listener:
        while True:
            conn, peer = listener.accept()
            with conn:
                conn.sendall(peer[0].encode("ascii") + b"\n")


def serve(server, ipv6):
    server.ipv6 = ipv6
    threading.Thread(target=server.serve_forever, daemon=True).start()


def check(host, kind, code, address):
    packet = b"\x21\x37" + struct.pack("!HHHHH", 0x100, 1, 0, 0, 0) + encode(host) + struct.pack("!HH", kind, 1)
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
        sock.settimeout(5)
        sock.sendto(packet, (address, 53))
        result, _ = sock.recvfrom(4096)
    assert result[:2] == packet[:2] and result[3] & 15 == code, (host, result[3] & 15, code)


if __name__ == "__main__":
    mode = sys.argv[1]
    match mode:
        case "dns":
            ipv6 = sys.argv[2]
            serve(socketserver.ThreadingUDPServer(("127.0.0.2", 53), Datagram), ipv6)
            serve(socketserver.ThreadingTCPServer(("127.0.0.2", 53), Stream), ipv6)
            threading.Event().wait()
        case "echo":
            serve(socketserver.ThreadingTCPServer(("203.0.113.7", 8080), Echo), "")
            echo(sys.argv[2])
        case "connect":
            host, family, source = sys.argv[2:]
            endpoints = socket.getaddrinfo(host, 8080, socket.AF_UNSPEC, socket.SOCK_STREAM)
            families = {item[0] for item in endpoints}
            if family == "four":
                assert families == {socket.AF_INET}, (host, families)
            else:
                assert socket.AF_INET6 in families, (host, families)
            with socket.create_connection((host, 8080), timeout=5) as conn:
                observed = conn.makefile("rb").readline().strip().decode("ascii")
                assert ipaddress.ip_address(observed) == ipaddress.ip_address(source), (host, observed, source)
        case "check":
            check(sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5])
