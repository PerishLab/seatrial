from pathlib import Path
import sys

from host import Host, emit
import flow
import lifecycle
import workspace
import interaction
from daily import config, entry, history, input, mouse, display, departure


def main():
    assert sys.flags.optimize == 0
    host = Host(sys.argv[2], sys.argv[3], Path(sys.argv[0]).resolve().parent)
    try:
        cases = {"workspace": workspace.run, "lifecycle": lifecycle.run, "flow": flow.run, "interaction": interaction.run}
        cases.update({"config": config.run, "entry": entry.run, "history": history.run, "input": input.run, "mouse": mouse.run, "display": display.run})
        cases["exit"] = departure.run
        cases[sys.argv[1]](host)
        host.stop()
        emit("owned host shutdown and direct-child reclamation", True)
    finally:
        host.close()


if __name__ == "__main__":
    main()
