import hashlib
import json
import pathlib
import subprocess


class Fixture:
    def __init__(self, repository, versions):
        self.repository = repository
        self.root = pathlib.Path("/trial/source") / repository
        self.root.mkdir(parents=True)
        self.records = []
        self.dispositions = []
        self.sequence = 0
        self.events = []
        self.products = {}
        for name, version in zip(["plumb", "ectropy"], versions, strict=True):
            binary = pathlib.Path("/opt/tools") / name
            actual = subprocess.check_output([binary, "--version"], text=True).strip()
            assert actual == name + " " + version, (name, actual, version)
            self.products[name] = {"version": actual, "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
        compiler = subprocess.check_output(["rustc", "-vV"], text=True)
        self.host = next(line[6:] for line in compiler.splitlines() if line.startswith("host: "))
        self.git("init", "-q", "-b", "topic")
        self.git("config", "user.name", "Seatrial")
        self.git("config", "user.email", "seatrial@example.invalid")
        self.git("remote", "add", "origin", "https://github.com/" + repository + ".git")
        self.write("Cargo.toml", '[package]\nname="probe"\nversion="0.0.0"\nedition="2024"\n[features]\nextra=[]\n')
        self.write("Cargo.lock", 'version=4\n[[package]]\nname="probe"\nversion="0.0.0"\n')
        self.write("src/lib.rs", "pub fn answer() -> u8 {\n    42\n}\npub fn data(value: &std::sync::Mutex<()>) {\n    drop(value.lock());\n}\n")
        self.write("protocol.txt", "synthetic collector interoperability input; native helpers are never executed\n")
        self.syntax("ban", "src/**")
        self.declare()
        self.stage()
        self.git("commit", "-q", "-m", "fixture")

    def call(self, *argv):
        return subprocess.run(argv, cwd=self.root, capture_output=True, text=True, timeout=120)

    def git(self, *argv):
        value = self.call("git", *argv)
        assert value.returncode == 0, value.stderr
        return value.stdout

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def stage(self):
        self.git("add", ".")

    def syntax(self, kind, path):
        self.write("ectropy.toml", '[scan]\ninclude=["src/**/*.rs"]\n[module]\nroots=["src"]\n[[' + kind + ']]\nsyntax="lock"\npaths=[' + json.dumps(path) + ']\n')

    def declare(self, enforce=True, features=None, ectropy=True):
        modes = ["default", "all-features"] if features is None else features
        text = '[workflow.hash.guard]\nrust=["Cargo.toml","Cargo.lock","src"]\n'
        if ectropy:
            text += 'ectropy=["ectropy.toml"]\n'
        text += '[guard.lock]\nenforce=' + str(enforce).lower() + '\ntarget=' + json.dumps(self.host) + '\nfeatures=' + json.dumps(modes) + '\n'
        for reference in self.dispositions:
            text += '\n[[guard.lock.review]]\nreference=' + json.dumps(reference) + '\nrole="interoperability"\nprotocol="synthetic compiler interoperability fixture, not a business exemption"\nhold="helpers are compiled but never executed by this fixture"\nrelease="no native acquisition is executed; this is not a cleanup safety proof"\nverification="this scene checks admission only through released CLIs"\n[guard.lock.review.evidence]\n"protocol.txt"=' + json.dumps(hashlib.sha256((self.root / "protocol.txt").read_bytes()).hexdigest()) + '\n'
        self.write("plumb.toml", text)

    def guard(self):
        self.sequence += 1
        diagnostic = self.root.parent / ("diagnostic." + self.root.name + "." + str(self.sequence) + ".jsonl")
        value = self.call("plumb", "guard", ".", "--json", "--diagnostic", str(diagnostic))
        self.events = [json.loads(line) for line in diagnostic.read_text().splitlines()] if diagnostic.exists() else []
        self.records.append({"operation": "guard", "exit": value.returncode, "events": [event["kind"] for event in self.events]})
        return value

    def review(self):
        found = set()
        for extra in [[], ["--all-features"]]:
            report = success(self.call("plumb", "lock", "resolve", ".", "--json", *extra))
            for finding in report["messages"]["findings"]:
                reference = finding["reference"]
                assert reference["state"] == "bound", reference
                found.add(reference["digest"])
            self.records.append({"operation": "resolve", "source": report["source"], "tree": report["tree"], "argv": report["argv"], "compiler": report["compiler"], "clippy": report["clippy"]})
        assert found, "native fixture must produce bound compiler references"
        self.dispositions = sorted(found)
        self.declare()
        self.stage()

    def record(self, status):
        print(json.dumps({"kind": "formal-tool conformance observation", "repository": self.repository, "ok": status, "products": self.products, "host": self.host, "operations": self.records, "guarded_fixture": "local synthetic repository, not actual GitHub landing", "qualification": "runner and trusted publisher own full identity/coverage/cleanup qualification"}), flush=True)


def success(value):
    assert value.returncode == 0, value.stdout[-2000:] + value.stderr[-6000:]
    return json.loads(value.stdout)


def reject(value, reason):
    assert value.returncode != 0 and not value.stdout, (value.returncode, value.stdout[-2000:])
    assert reason in value.stderr, value.stderr[-6000:]


def action(proof, name):
    return next(value for value in proof["actions"] if value["name"] == name)
