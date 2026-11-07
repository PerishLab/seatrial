from pathlib import Path
import sys

from host import Host, emit
import flow
import lifecycle
import workspace


def main():
    assert sys.flags.optimize == 0
    host = Host(sys.argv[2], sys.argv[3], Path(sys.argv[0]).resolve().parent)
    try:
        cases = {"workspace": workspace.run, "lifecycle": lifecycle.run, "flow": flow.run}
        cases[sys.argv[1]](host)
        host.stop()
        emit("owned host shutdown and direct-child reclamation", True)
    finally:
        host.close()


if __name__ == "__main__":
    main()
