import json
from pathlib import Path
import subprocess


class Client:
    def __init__(self, root):
        self.root = Path(root)
        self.root.mkdir(parents=True, exist_ok=True)
        self.root.chmod(0o700)
        self.home = self.root / "ensign"
        self.home.mkdir(mode=0o700)
        self.base = "http://127.0.0.1:8080/api"
        self.env = {"HOME": str(self.root / "home"), "PATH": "/usr/local/bin:/usr/bin:/bin", "ENSIGN_HOME": str(self.home), "ENSIGN_URL": self.base}

    def invoke(self, args, code=0, data=None):
        result = subprocess.run(args, env=self.env, input=data, text=True, capture_output=True, timeout=45, cwd=self.root)
        assert result.returncode == code, "public CLI outcome disagrees: " + str(args[:3]) + " exit=" + str(result.returncode) + " error=" + result.stderr
        return result.stdout

    def login(self, login, password):
        self.invoke(["/opt/ensign", "login", login], data=password + "\n")

    def binding(self, name, values):
        path = self.root / (name + ".json")
        body = {"source": self.base, "issuer": "https://identity.example/api", "subject": "fixture-subject", "audience": "cloud", "account": "100", "provider": "acs:ram::100:oidc-provider/identity", "role": "acs:ram::100:role/operator", "id": "300", "session": "fixture-session", "duration": 900}
        body.update(values)
        path.write_text(json.dumps(body))
        return path

    def call(self, state, words, code=0):
        return self.invoke(["/opt/runseal", ":", "@alibaba", "--state", str(state), *words], code)

    def acquire(self, name, values, code):
        state = self.root / "roles" / name / "state.json"
        state.parent.mkdir(mode=0o700, parents=True)
        binding = self.binding(name, values)
        self.call(state, ["auth", "federate", "--binding-file", str(binding), "--ensign-home", str(self.home)], code)
        return state

    def status(self, state):
        value = json.loads(self.call(state, ["auth", "status"]))
        assert value["credentialPresent"] is False
        assert value["identity"] is None
        return value
