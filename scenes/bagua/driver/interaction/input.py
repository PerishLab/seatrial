from daily.common import until
from host import emit
from wire import text, wait
from .common import Trial


def navigation(host):
    trial = Trial(host, "interaction-input")
    try:
        pane = trial.pane()
        selected = trial.state()["selected"]
        trial.view.write(b"\x1b9FIRST-FRESH\x1b9SECOND-FRESH\x1b1\x11\tNOOP-FRESH")
        wait(lambda: "NOOP-FRESH" in text(trial.client.screen(pane)))
        assert trial.state()["selected"] == selected
        assert all(value in text(trial.client.screen(pane)) for value in ["FIRST-FRESH", "SECOND-FRESH", "NOOP-FRESH"])
        trial.view.resize(120, 3)
        wait(lambda: trial.state()["sizing"]["Automatic"]["rows"] == 2)
        trial.view.write(b"\x11-NEVER-OLD")
        until(trial.view, "Split top/bottom:")
        assert "NEVER-OLD" not in text(trial.client.screen(pane))
        trial.view.write(b"REFUSAL-FRESH")
        wait(lambda: "REFUSAL-FRESH" in text(trial.client.screen(pane)))
        assert "NEVER-OLD" not in text(trial.client.screen(pane))
        trial.view.detach(b"\x1bq")
        emit("invalid index and no-op navigation preserve following input; refused split discards dependent bytes and fresh typing needs no Escape", True)
    finally:
        trial.close()


def native(host):
    trial = Trial(host, "interaction-native", "transient")
    try:
        first = trial.pane()
        (host.root / "transient-python").unlink()
        trial.view.write(b"\x11\\START-FAIL-NEVER")
        until(trial.view, "Start pane:")
        assert len(trial.bodies()) == 2
        assert all("START-FAIL-NEVER" not in body for body in trial.bodies().values())
        trial.view.write(b"\x11\tNATIVE-FRESH")
        wait(lambda: "NATIVE-FRESH" in text(trial.client.screen(first)))
        assert all("START-FAIL-NEVER" not in body for body in trial.bodies().values())
        trial.view.detach(b"\x1bq")
        emit("failed native exec retains the applied failed pane, discards its dependent input and accepts fresh explicit focus without Escape", True)
    finally:
        trial.close()


def run(host):
    navigation(host)
    native(host)
