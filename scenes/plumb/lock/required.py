import sys
from fixture import Fixture, action, reject, success


def run(repository):
    fixture = Fixture(repository, sys.argv[1:])
    passed = False
    try:
        first = success(fixture.guard())
        action(first, "guard/lock")
        original = (fixture.root / "plumb.toml").read_text()
        fixture.declare(enforce=False)
        fixture.stage()
        reject(fixture.guard(), "requires guard.lock.enforce=true")
        fixture.write("plumb.toml", original.split("[guard.lock]")[0])
        fixture.stage()
        reject(fixture.guard(), "requires guard.lock.enforce=true")
        for features in [["default"], ["all-features"]]:
            fixture.declare(features=features)
            fixture.stage()
            reject(fixture.guard(), "requires its exact carried feature coverage")
        fixture.declare(ectropy=False)
        fixture.stage()
        reject(fixture.guard(), "requires both guard/rust and guard/ectropy")
        fixture.declare()
        fixture.stage()
        assert success(fixture.guard()) == first
        passed = True
    finally:
        fixture.record(passed)


for repository in ["PerishLab/plumb", "PerishLab/runseal"]:
    run(repository)
