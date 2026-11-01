import sys
from fixture import Fixture, action, reject, success

fixture = Fixture("Example/probe", sys.argv[1:])
passed = False
try:
    first = success(fixture.guard())
    for name in ["guard/lock", "guard/ectropy"]:
        action(first, name)
    assert success(fixture.guard()) == first
    assert any(event["kind"] == "guard.reused" for event in fixture.events), "exact proof reuse was not observed"
    fixture.write("outside.txt", "outside cropped workflow hashes\n")
    fixture.stage()
    changed = success(fixture.guard())
    assert action(changed, "guard/rust") == action(first, "guard/rust")
    for name in ["guard/lock", "guard/ectropy"]:
        assert action(changed, name) != action(first, name)
    fixture.write("src/lib.rs", "pub fn native(file: &std::fs::File) {\n    let _ = file.try_lock();\n}\n")
    fixture.stage()
    reject(fixture.guard(), "lock disposition mismatch")
    fixture.review()
    reviewed = success(fixture.guard())
    protocol = (fixture.root / "protocol.txt").read_text()
    fixture.write("protocol.txt", "changed protocol without a fresh disposition\n")
    fixture.stage()
    reject(fixture.guard(), "evidence is stale")
    fixture.write("protocol.txt", protocol)
    fixture.stage()
    assert success(fixture.guard()) == reviewed
    fixture.write("src/lib.rs", "pub fn native(file: &std::fs::File) {\n    let _ = std::fs::File::try_lock(file);\n}\n")
    fixture.stage()
    fixture.review()
    checked = success(fixture.call("plumb", "lock", "check", ".", "--json"))
    assert checked["consistent"] and checked["guard"] is False
    syntax = fixture.call("ectropy", ".")
    assert syntax.returncode == 1 and "lock" in syntax.stdout, syntax.stdout + syntax.stderr
    reject(fixture.guard(), "ectropy . failed")
    fixture.syntax("grant", "src/lib.rs")
    fixture.stage()
    granted = success(fixture.guard())
    action(granted, "guard/lock")
    action(granted, "guard/ectropy")
    fixture.syntax("grant", "src")
    fixture.stage()
    reject(fixture.guard(), "regular file")
    fixture.syntax("grant", "src/lib.rs")
    fixture.stage()
    assert success(fixture.guard()) == granted
    passed = True
finally:
    fixture.record(passed)
