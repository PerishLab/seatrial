import hashlib
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
    manifest = (fixture.root / "Cargo.toml").read_text()
    fixture.write("Cargo.toml", manifest + '\n[dependencies]\ncompanion={path="src/companion"}\n')
    fixture.write("src/companion/Cargo.toml", '[package]\nname="companion"\nversion="0.0.0"\nedition="2024"\n')
    fixture.write("src/companion/src/lib.rs", "pub fn observe() {}\n")
    fixture.write("Cargo.lock", 'version=4\n[[package]]\nname="probe"\nversion="0.0.0"\ndependencies=["companion"]\n[[package]]\nname="companion"\nversion="0.0.0"\n')
    fixture.write("src/lib.rs", "pub fn native(file: &std::fs::File) {\n    let _ = file.try_lock();\n}\n")
    fixture.stage()
    reject(fixture.guard(), "lock disposition mismatch")
    fixture.review()
    reviewed = success(fixture.guard())
    before = success(fixture.call("plumb", "lock", "check", ".", "--json"))
    assert before["schema"] == "plumb.lock-review/v2", "v2 source review and lockfile reports required"
    assert before["consistent"] and before["guard"] is False and before["references"]
    declaration = (fixture.root / "plumb.toml").read_bytes()
    previous = hashlib.sha256((fixture.root / "Cargo.lock").read_bytes()).hexdigest()
    companion = (fixture.root / "src/companion/Cargo.toml").read_text()
    fixture.write("src/companion/Cargo.toml", companion.replace('version="0.0.0"', 'version="0.0.1"'))
    lock = (fixture.root / "Cargo.lock").read_text()
    fixture.write("Cargo.lock", lock.replace('name="companion"\nversion="0.0.0"', 'name="companion"\nversion="0.0.1"'))
    fixture.stage()
    after = success(fixture.call("plumb", "lock", "check", ".", "--json"))
    assert after["consistent"] and after["guard"] is False
    assert before["references"] == after["references"] and before["declaration"] == after["declaration"]
    assert (fixture.root / "plumb.toml").read_bytes() == declaration, "product rewrote human dispositions"
    current = hashlib.sha256((fixture.root / "Cargo.lock").read_bytes()).hexdigest()
    assert len(before["reports"]) == len(after["reports"]) == 2
    assert ["--all-features" in report["argv"] for report in after["reports"]] == [False, True]
    for old, new in zip(before["reports"], after["reports"], strict=True):
        assert old["schema"] == new["schema"] == "plumb.lock-resolution/v2"
        assert old["tree"] != new["tree"] and old["lockfile"] == previous and new["lockfile"] == current
        assert old["lockfile"] != new["lockfile"]
    refreshed = success(fixture.guard())
    assert refreshed["tree"] != reviewed["tree"], "dependency world reused the complete protected tree"
    for name in ["guard/lock", "guard/ectropy"]:
        assert action(refreshed, name) != action(reviewed, name), f"{name} did not refresh for the dependency world"
    assert not any(event["kind"] == "guard.reused" for event in fixture.events)
    fixture.records.append({"operation": "dependency-world", "same_dispositions": True, "ordinary_rust_reused": action(refreshed, "guard/rust") == action(reviewed, "guard/rust"), "references": after["references"], "before": [{"tree": report["tree"], "lockfile": report["lockfile"]} for report in before["reports"]], "after": [{"tree": report["tree"], "lockfile": report["lockfile"]} for report in after["reports"]]})
    reviewed = refreshed
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
