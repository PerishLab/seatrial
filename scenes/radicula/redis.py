import hashlib
import json
import os
import pathlib
import queue
import subprocess
import threading
import time

lane = os.environ["SEATRIAL_LANE"]
work = pathlib.Path(os.environ["SEATRIAL_WORK"])
scene = pathlib.Path(os.environ["SEATRIAL_SCENE"])
binary = work / "target/debug/seatrial-radicula"
image = os.environ["SEATRIAL_IMAGE_REDIS"]
runtime = os.environ["SEATRIAL_IMAGE_DRIVER"]
checks = []
clients = []
labels = ["--label", "perish.owner=seatrial", "--label", "perish.lane=" + lane]

def docker(*args):
    return subprocess.check_output(["docker", *args], text=True, timeout=30).strip()

def until(observe, description, timeout=8):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        result = observe()
        if result:
            return result
        time.sleep(0.01)
    raise AssertionError("state deadline: " + description)

def start(name, backend=image, *args):
    return docker("run", "--detach", "--name", lane + "-" + name, "--network", lane,
        "--network-alias", name + "." + lane + ".internal", *labels,
        "--mount", "type=bind,src=" + str(work) + ",dst=/trial", backend, *args)

def address(name):
    return docker("inspect", "--format", "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}", lane + "-" + name) + ":8080"

def redis(*args):
    return docker("exec", lane + "-redis", "redis-cli", "-p", "8080", "--raw", *args)

class Client:
    def __init__(self, namespace, renewal="100", endpoint="redis"):
        self.name = lane + "-client-" + str(len(clients))
        args = ["docker", "run", "--rm", "--interactive", "--name", self.name, "--network", lane, *labels,
            "--mount", "type=bind,src=" + str(binary) + ",dst=/opt/conformance,readonly", runtime,
            "/opt/conformance", address(endpoint), namespace, "40", "1000", renewal]
        self.process = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.replies = queue.Queue()
        def read():
            for line in self.process.stdout:
                self.replies.put(line.strip())
            self.replies.put("EOF")
        threading.Thread(target=read, daemon=True).start()
        clients.append(self)
        self.epoch = self.receive().removeprefix("ready ")
        assert len(self.epoch) == 40, self.epoch
    def receive(self):
        response = self.replies.get(timeout=10)
        assert response != "EOF", self.process.stderr.read()
        return response
    def call(self, text, prefix=None):
        self.process.stdin.write(text + "\n")
        self.process.stdin.flush()
        response = self.receive()
        if prefix is not None:
            assert response.startswith(prefix), (text, response, prefix)
        return response
    def close(self):
        if self.process.poll() is None:
            self.process.stdin.close()
            self.process.wait(timeout=10)
        assert self.process.returncode == 0, self.process.stderr.read()

def key(namespace, name, suffix):
    return "radicula:lock:v1:" + namespace.encode().hex() + ":" + name.encode().hex() + ":" + suffix

def expiry(namespace, name):
    values = redis("ZRANGE", key(namespace, name, "leases"), "0", "0", "WITHSCORES").splitlines()
    assert len(values) == 2, values
    return int(values[1])

def expired(deadline):
    clock = redis("TIME").splitlines()
    return int(clock[0]) * 1000 + int(clock[1]) // 1000 >= deadline

try:
    start("redis", image, "redis-server", "--port", "8080", "--save", "", "--appendonly", "no", "--maxmemory-policy", "noeviction")
    until(lambda: redis("PING") == "PONG", "Redis readiness")
    a, b, c = Client("capacity"), Client("capacity"), Client("capacity")
    for client in [a, b, c]:
        client.call("gate work 2", "bound")
    a.call("take work a exclusive", "held")
    b.call("take work b exclusive", "held")
    c.call("take work c exclusive", "error Busy")
    a.call("drop a", "released")
    c.call("take work c exclusive", "held")
    b.call("drop b", "released")
    c.call("drop c", "released")
    checks.append("independent processes share remote capacity")
    d = Client("capacity")
    d.call("gate work 1", "bound")
    d.call("take work d exclusive", "error Conflict")
    checks.append("remote policy conflicts refuse")
    a.call("gate readers readers", "bound")
    b.call("gate readers readers", "bound")
    a.call("take readers a shared", "held")
    b.call("take readers b shared", "held")
    c.call("gate readers readers", "bound")
    c.call("take readers c exclusive", "error Busy")
    a.call("drop a", "released")
    b.call("drop b", "released")
    c.call("take readers c exclusive", "held")
    a.call("take readers a shared", "error Busy")
    c.call("drop c", "released")
    checks.append("distributed shared readers exclude writer")
    a.call("gate scope 1", "bound")
    a.call("guard scope error", 'result Ok(Err("business"))')
    a.call("guard scope panic", "panic true")
    a.call("macro scope", "result Ok(42)")
    a.call("take scope scope exclusive", "held")
    first = expiry("capacity", "scope")
    until(lambda: expiry("capacity", "scope") > first + 300, "renewal advances ownership")
    a.call("check scope", "valid")
    a.call("drop scope", "released")
    checks.append("business error panic macro and managed renewal clean up")
    old, new = Client("expiry", "none"), Client("expiry")
    for client in [old, new]:
        client.call("gate work 1", "bound")
    before = old.call("take work old exclusive", "held").split()
    until(lambda: expired(expiry("expiry", "work")), "nonrenewed lease expires")
    after = new.call("take work new exclusive", "held").split()
    assert before[1] == after[1] and int(after[2]) > int(before[2])
    old.call("check old", "error Lost")
    old.call("drop old", "error Lost")
    new.call("check new", "valid")
    old.call("take work old exclusive", "error Uncertain")
    new.call("drop new", "released")
    checks.append("expiry keeps old execution alive but old release cannot remove newer owner")
    paused, winner = Client("paused"), Client("paused")
    for client in [paused, winner]:
        client.call("gate work 1", "bound")
    paused.call("take work old exclusive", "held")
    docker("pause", paused.name)
    until(lambda: expired(expiry("paused", "work")), "paused lease expires")
    winner.call("take work new exclusive", "held")
    docker("unpause", paused.name)
    paused.call("check old", "error Lost")
    paused.call("drop old", "error Lost")
    winner.call("check new", "valid")
    winner.call("drop new", "released")
    checks.append("process pause loses lease without terminating old execution")
    start("proxy", runtime, "/trial/target/debug/seatrial-radicula", "proxy", "0.0.0.0:8080", address("redis"), "/trial/fault")
    def ready():
        try:
            return docker("exec", lane + "-proxy", "sh", "-c", "grep -qi 00000000:1F90 /proc/net/tcp") == ""
        except subprocess.CalledProcessError:
            return False
    until(ready, "proxy process readiness")
    for action in ["acquire", "release", "renew"]:
        client = Client("unknown-" + action, endpoint="proxy")
        client.call("gate work 1", "bound")
        if action != "acquire":
            client.call("take work held exclusive", "held")
        observed = work / "fault.observed"
        observed.unlink(missing_ok=True)
        (work / "fault").write_text(action)
        command = {"acquire": "take work held exclusive", "release": "drop held", "renew": "check held"}[action]
        expected = {"acquire": "error Unconfirmed", "release": "error Uncertain", "renew": "error Lost"}[action]
        if action == "renew":
            until(lambda: observed.exists(), "renewal reply dropped after server response")
        client.call(command, expected)
        if action == "renew":
            client.call("drop held")
        assert observed.read_text() == action
        client.call("take work next exclusive", "error Uncertain")
        checks.append(action + " reply loss is observed and prevents automatic recovery")
    restart = Client("restart")
    restart.call("gate work 1", "bound")
    restart.call("take work old exclusive", "held")
    docker("restart", lane + "-redis")
    until(lambda: redis("PING") == "PONG", "Redis restart readiness")
    until(lambda: restart.call("check old").startswith("error Lost"), "restart invalidates old lease")
    fresh = Client("restart")
    assert fresh.epoch != restart.epoch
    fresh.call("gate work 1", "bound")
    fresh.call("take work new exclusive", "held")
    restart.call("drop old", "error Epoch")
    fresh.call("check new", "valid")
    fresh.call("drop new", "released")
    checks.append("restart changes epoch and old release cannot affect new domain")
    def material(reference):
        observed = json.loads(docker("image", "inspect", reference))[0]
        return {"reference": reference, "id": observed["Id"], "digests": observed["RepoDigests"]}
    report = {"checks": checks, "identity": json.loads((work / "conformance-identity.json").read_text()), "redisImage": material(image), "runtimeImage": material(runtime), "passed": True}
    (work / "redis-report.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report))
finally:
    for client in clients:
        subprocess.run(["docker", "unpause", client.name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10)
        client.close()
