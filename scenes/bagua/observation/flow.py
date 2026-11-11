import json
import re
import signal
import subprocess
import time

from daily.common import legacy as config, panes, profile, settle, until
from daily.entry import describe, finished, shell
from daily import departure
from host import alive, emit, sessions
from terminal import Terminal
from wire import wait
from workspace import join
from service import payloads, trace

MARKERS = ["PRIVATE-ARG-CANARY", "PRIVATE-ENV-CANARY", "PRIVATE-CWD-CANARY", "PRIVATE-INPUT-CANARY", "private-name-canary"]


def settings(host, endpoint):
    return config(host, profile() + "\n[header]\nenabled=true\ncomponents=['pane','command','cwd']\n[locus]\nenabled=true\nendpoint=" + json.dumps(endpoint) + "\n", "observed.toml")


def invocations(atoms):
    return {trace(atom) for atom in payloads(atoms, "cli.start")}


def identity(atoms, version):
    typed = [atom for atom in atoms if any(str(atom.get("payload", {}).get("event", "")).startswith(prefix) for prefix in ["pane.", "request", "client.", "terminal."])]
    assert typed
    for atom in typed:
        assert atom["context"].get("bagua.version") == version
        assert re.fullmatch(r"[0-9a-f]{40}", atom["context"].get("bagua.source", ""))
    assert not any(marker in json.dumps(atoms) for marker in MARKERS)


def normal(host, service, configured):
    session, survivor, owner = departure.layout(host)
    departure.interactive(host, survivor, configured)
    owner.workspace({"Close": {"Session": session}})
    assert not alive(survivor)
    before = invocations(service.events())
    departure.metadata(host, configured)
    atoms = service.until(lambda: metadata(service.events(), before))
    assert all(payloads(atoms, name) for name in ["pane.reaped", "pane.release", "pane.retire", "terminal.finish", "client.return"])
    assert any(atom["payload"]["code"] == 9 for atom in payloads(atoms, "pane.reaped"))
    assert all(atom["payload"]["restored"] for atom in payloads(atoms, "terminal.finish"))
    assert all(atom["payload"]["code"] == 0 for atom in payloads(atoms, "cli.finish"))
    for atom in payloads(atoms, "pane.retire"):
        pane = atom["payload"]["pane"]
        group = [entry for entry in atoms if entry["context"] == atom["context"] and entry.get("payload", {}).get("pane") == pane]
        names = [entry["payload"]["event"] for entry in group]
        assert names.count("pane.retire") == 1 and names.index("pane.reaped") < names.index("pane.release") < names.index("pane.retire")
    emit("observation preserves all-exit, real reaping, TTY restoration and immediate metadata refusal without Finished", {"metadata_traces": sorted(invocations(atoms) - before), "atoms": len(atoms)})


def metadata(atoms, before):
    added = invocations(atoms) - before
    rejected = [atom for atom in payloads(atoms, "request") if trace(atom) in added and atom["payload"]["operation"] == "describe" and atom["payload"]["stage"] == "refused"]
    if not rejected or not all(any(trace(atom) == trace(refused) for atom in payloads(atoms, "cli.finish")) for refused in rejected):
        return None
    for refused in rejected:
        assert not any(atom["context"] == refused["context"] and atom["payload"].get("request") == refused["payload"]["request"] and atom["payload"]["stage"] in ("finished", "failed") for atom in payloads(atoms, "request"))
    return atoms


def privacy(host, service, configured):
    directory = host.root / "PRIVATE-CWD-CANARY"
    directory.mkdir(mode=0o700)
    args = [*configured, "enter", "private-name-canary", "--executable", "/bin/sh", "--cwd", str(directory), "--arg=-i", "--arg=-s", "--arg=PRIVATE-ARG-CANARY", "--env", "PATH=/usr/bin:/bin", "--env", "TERM=xterm-256color", "--env", "PRIVATE=PRIVATE-ENV-CANARY"]
    view = Terminal(host, args)
    client = host.wire()
    try:
        until(view, "control")
        value = next(value for value in sessions(client) if value["name"] == "private-name-canary")
        join(client, value["id"])
        pid = describe(client, panes(value)[0]["id"])["pid"]
        host.children.append(pid)
        view.write(b"printf 'ARG:%s|%s\\n' \"$0\" \"${1-}\"; pwd; printenv PRIVATE; if printenv LOCUS_PARENT >/dev/null; then printf '\\125NEXPECTED_PARENT\\n'; else printf '\\120ARENT_ABSENT\\n'; fi\r")
        until(view, "PRIVATE-ENV-CANARY")
        until(view, "PARENT_ABSENT")
        until(view, "PRIVATE-ARG-CANARY")
        assert "UNEXPECTED_PARENT" not in view.screen()
        view.write(b"printf 'PRIVATE-INPUT-CANARY\\n'\r")
        until(view, "PRIVATE-INPUT-CANARY")
        view.write(b"\x11[")
        until(view, "Copy")
        view.write(b"\x1b")
        settle(view, 0.12)
        assert "Copy" not in view.screen()
        view.write(b"exit\r")
        finished(view, 0)
        wait(lambda: not sessions(client))
        assert not alive(pid)
    finally:
        view.close()
    emit("actual program arguments, environment, cwd, name and TTY input stay outside observation", True)


def crash(host, service, configured):
    service.until(lambda: len(payloads(service.events(), "cli.finish")) >= 9)
    before = invocations(service.events())
    view = Terminal(host, [*configured, "enter", "crash", *shell(host)])
    client = host.wire()
    try:
        until(view, "control")
        value = next(value for value in sessions(client) if value["name"] == "crash")
        session = value["id"]
        join(client, session)
        pid = describe(client, panes(value)[0]["id"])["pid"]
        host.children.append(pid)
        added = service.until(lambda: invocations(service.events()) - before)
        assert len(added) == 1
        killed = next(iter(added))
        view.process.send_signal(signal.SIGKILL)
        assert view.process.wait(timeout=5) == -signal.SIGKILL
        wait(lambda: next(value for value in sessions(client) if value["id"] == session)["control"]["owner"] is None)
        assert host.process.poll() is None and alive(pid)
        atoms = service.events()
        assert not any(trace(atom) == killed for atom in payloads(atoms, "cli.finish"))
        assert not any(trace(atom) == killed for atom in payloads(atoms, "terminal.finish"))
        client.workspace({"Acquire": {"session": session, "fixed": None}})
        client.workspace({"Close": {"Session": session}})
        assert not alive(pid)
        emit("owned killed client retains an unfinished trace while host and application survive", {"trace": killed})
    finally:
        view.close()


def transparent(host, configured):
    timings = {}
    outputs = []
    for name, controls in [("off", ["--no-config"]), ("on", configured)]:
        samples = []
        for _ in range(4):
            start = time.monotonic()
            result = subprocess.run([host.binary, *controls, "--json", "capabilities"], env=host.env, capture_output=True, timeout=15)
            samples.append(time.monotonic() - start)
            assert result.returncode == 0 and result.stderr == b""
            outputs.append(result.stdout)
        timings[name] = sorted(samples)
    assert all(output == outputs[0] for output in outputs)
    emit("observation off/on keeps business stdout and status; measured startup cost only", timings)


def outage(host, service, configured):
    before = len(payloads(service.events(), "cli.finish"))
    service.stop()
    samples = []
    for _ in range(4):
        result = subprocess.run([host.binary, *configured, "--json", "capabilities"], env=host.env, capture_output=True, timeout=15)
        assert result.returncode == 0
        json.loads(result.stdout)
        assert result.stderr == b""
        samples.append(result.stdout)
    cold = subprocess.run([host.binary, *configured, "--json", "capabilities"], env={**host.env, "XDG_STATE_HOME": str(host.root / "unregistered-state")}, capture_output=True, timeout=15)
    notes = [json.loads(line) for line in cold.stderr.splitlines()]
    assert cold.returncode == 0 and cold.stdout == samples[0]
    assert notes and all(note["diagnostic"]["source"] == "bagua.locus" for note in notes)
    spool = host.home / ".local/state/bagua/locus"
    size = sum(path.stat().st_size for path in spool.rglob("*") if path.is_file())
    assert 0 < size < 32 * 1024 * 1024
    assert len(set(samples)) == 1 and host.process.poll() is None
    service.start()
    atoms = service.until(lambda: restored(service, before + 4))
    emit("registered spool drains after owned service restart; fresh unregistered spool diagnoses without changing business output", {"observed_spool_bytes": size, "recovered_finishes": len(payloads(atoms, "cli.finish")) - before, "unregistered_diagnostics": len(notes)})


def restored(service, count):
    atoms = service.events()
    return atoms if len(payloads(atoms, "cli.finish")) >= count else None
