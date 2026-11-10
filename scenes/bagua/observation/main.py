import json
from pathlib import Path
import subprocess
import sys
import time
from types import SimpleNamespace

sys.path.insert(0, "/scene/driver")
from host import Host, emit
from service import Service, payloads, trace
import flow


def main():
    assert sys.flags.optimize == 0
    root = Path(sys.argv[1])
    bagua = SimpleNamespace(binary=sys.argv[2], version=sys.argv[5])
    locus = SimpleNamespace(binary=sys.argv[3], version=sys.argv[6])
    source = Path("/scene/driver")
    service = Service(root / "api", locus, sys.argv[4])
    host = None
    try:
        actual = subprocess.run([bagua.binary, "--version"], capture_output=True, text=True, check=True, timeout=10).stdout.strip()
        assert actual == "bagua " + bagua.version, actual
        directory = root / "bagua"
        directory.mkdir(mode=0o700)
        (directory / "server.toml").write_text("[locus]\nenabled=true\nendpoint=" + json.dumps(service.endpoint) + "\n")
        host = Host(directory, bagua.binary, source)
        configured = flow.settings(host, service.endpoint)
        selected = host.home / ".config/bagua"
        selected.mkdir(mode=0o700, parents=True)
        (selected / "bagua.toml").write_text((directory / "server.toml").read_text())
        flow.transparent(host, configured)
        flow.normal(host, service, configured)
        flow.privacy(host, service, configured)
        flow.crash(host, service, configured)
        flow.outage(host, service, configured)
        host.stop()
        atoms = service.until(lambda: complete(service.events()))
        flow.identity(atoms, bagua.version)
        body, headers = service.request("/api/v1/atoms?producer=bagua")
        floor = int(headers["locus-retained-from"])
        assert 0 < floor < time.time_ns()
        prior, _ = service.query(to=floor - 1)
        assert not prior
        traces = sorted({trace(atom) for atom in payloads(atoms, "cli.start")})
        owned = next(trace(atom) for atom in payloads(atoms, "cli.start") if atom["context"].get("bagua.command") == "host")
        selected, _ = service.query(role="locus.trace", key=owned)
        assert selected and all(trace(atom) == owned for atom in selected)
        emit("public API query records actual scoped/versioned facts, declared retention floor and gaps without inventing history", {"version": bagua.version, "source": sorted({atom["context"]["bagua.source"] for atom in atoms if "bagua.source" in atom["context"]}), "traces": traces, "retained_from": floor, "atoms": len(atoms), "bytes": len(body)})
    finally:
        if host is not None:
            host.close()
        service.close()


def complete(atoms):
    stopped = [atom for atom in payloads(atoms, "host.stop") if atom["payload"]["success"]]
    if not stopped or not any(trace(atom) == trace(stopped[0]) for atom in payloads(atoms, "cli.finish")):
        return None
    return atoms


if __name__ == "__main__":
    main()
