import os
import subprocess
import sys


class Lane:
    def __init__(self):
        self.name = os.environ["SEATRIAL_LANE"]
        self.store = os.environ["SEATRIAL_VOLUME"]
        self.scene = os.environ["SEATRIAL_SCENE"]
        self.image = os.environ["SEATRIAL_IMAGE"]
        self.binary = os.environ["SEATRIAL_BINARY_ENSIGN"]
        self.service = os.environ.get("SEATRIAL_IMAGE_ENSIGN")
        self.runseal = os.environ["SEATRIAL_BINARY_RUNSEAL"]
        self.environment = {"PATH": os.environ["PATH"], "HOME": os.environ["HOME"]}
        self.labels = ["--label", "perish.owner=seatrial", "--label", "perish.lane=" + self.name]

    def invoke(self, args):
        result = subprocess.run(["docker", *args], env=self.environment, capture_output=True, text=True, timeout=180)
        if result.returncode:
            print(result.stderr, file=sys.stderr)
        assert result.returncode == 0, "isolated Docker operation failed: " + str(args[:2])
        return result.stdout

    def mount(self, source, target):
        return ["--mount", "type=bind,src=" + str(source) + ",dst=" + target + ",readonly"]

    def persistent(self, source, target):
        return ["--mount", "type=volume,src=" + source + ",dst=" + target]

    def volume(self, suffix):
        name = self.name + "-ensign-" + suffix
        self.invoke(["volume", "create", *self.labels, name])
        return name

    def server(self):
        volumes = {name: self.volume(name) for name in ["estate", "signing", "sudo", "consumer"]}
        assert self.service and "@sha256:" in self.service
        user = self.invoke(["run", "--rm", "--network", self.name, *self.labels, "--entrypoint", "id", self.service, "-u"]).strip()
        assert user.isdigit() and user != "0"
        initialize = ["run", "--rm", "--network", self.name, *self.labels]
        for name, volume in volumes.items():
            initialize.extend(self.persistent(volume, "/" + name))
        self.invoke([*initialize, "--entrypoint", "sh", self.image, "-eu", "-c", 'chown "$1:$1" /estate /signing /sudo /consumer; chmod 700 /estate /signing /sudo /consumer', "sh", user])
        common = ["--network", self.name, *self.labels]
        for name in ["estate", "signing"]:
            common.extend(self.persistent(volumes[name], "/" + name))
        for value in ["API_STORE_KIND=file", "API_STORE_PATH=/estate/estate.db", "API_ISS=https://identity.example/api", "API_LISTEN_HOST=0.0.0.0", "API_LISTEN_PORT=8080"]:
            common.extend(["--env", value])
        bootstrap = ["run", "--rm", "--name", self.name + "-ensign-bootstrap", *common]
        for name in ["sudo", "consumer"]:
            bootstrap.extend(self.persistent(volumes[name], "/" + name))
        self.invoke([*bootstrap, self.service, "bootstrap", "/tmp", "--artifact", "sudo=file:/sudo/token", "--artifact", "signing=file:/signing/key", "--artifact", "key:cloud=file:/consumer/token"])
        self.invoke(["run", "-d", "--name", self.name + "-ensign-api", "--network-alias", "ensign." + self.name + ".internal", *common, self.service, "serve", "/tmp", "--artifact", "signing=file:/signing/key"])
        return volumes["sudo"]

    def run(self, mode):
        assert mode in ["actual", "claims"]
        args = ["run", "--rm", "--name", self.name + "-ensign-" + mode, "--network", self.name, "--network-alias", "issuer." + self.name + ".internal", *self.labels]
        args.extend(self.persistent(self.store, "/trial"))
        for source, target in [(self.scene, "/scene"), (self.binary, "/opt/ensign"), (self.runseal, "/opt/runseal")]:
            args.extend(self.mount(source, target))
        if mode == "actual":
            args.extend(self.persistent(self.server(), "/sudo"))
        args.extend(["--entrypoint", "python3", self.image, "-B", "/scene/" + mode + ".py"])
        if mode == "actual":
            args.append("ensign." + self.name + ".internal")
        print(self.invoke(args), end="")


if __name__ == "__main__":
    Lane().run(sys.argv[1])
