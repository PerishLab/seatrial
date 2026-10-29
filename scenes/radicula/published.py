import hashlib
import io
import json
import pathlib
import subprocess
import tarfile


def raw(url):
    return subprocess.check_output(["curl", "--fail", "--silent", "--show-error", "--retry", "2", "--retry-all-errors", "--max-time", "30", url])


def latest(name):
    assert len(name) >= 4
    data = raw("https://cargo.perish.uk/" + name[:2] + "/" + name[2:4] + "/" + name)
    entries = [json.loads(line) for line in data.splitlines() if line]
    eligible = [entry for entry in entries if not entry["yanked"] and "-" not in entry["vers"] and entry["vers"].startswith("0.")]
    return max(eligible, key=lambda entry: tuple(map(int, entry["vers"].split("."))))


def verify(package, resolution):
    name = package["name"]
    index = latest(name)
    assert package["version"] == index["vers"]
    lock = next(entry for entry in resolution if entry["name"] == name)
    assert lock["checksum"] == index["cksum"]
    assert package["source"] is not None
    archive = raw("https://cargo.perish.uk/crates/" + name + "/" + index["vers"] + "/" + index["cksum"] + ".crate")
    assert hashlib.sha256(archive).hexdigest() == index["cksum"]
    installed = pathlib.Path(package["manifest_path"]).parent
    prefix = name + "-" + index["vers"] + "/"
    files = 0
    with tarfile.open(fileobj=io.BytesIO(archive)) as material:
        for member in material.getmembers():
            if not member.isfile():
                continue
            assert member.name.startswith(prefix)
            relative = pathlib.Path(member.name.removeprefix(prefix))
            assert not relative.is_absolute() and ".." not in relative.parts
            assert (installed / relative).read_bytes() == material.extractfile(member).read()
            files += 1
    distribution = json.loads(raw("https://releases.radicula.perish.uk/v1/releases/stable/v" + index["vers"] + "/distribution.json"))
    assert distribution["state"] == "complete"
    vcs = json.loads((installed / ".cargo_vcs_info.json").read_text())
    snapshot = distribution["resolution"]["cargo"]
    assert snapshot["schema"] == "wharf.release.snapshot/v1"
    for field in ["repository", "marker", "commit", "tree"]:
        assert snapshot["source"][field] == distribution[field]
    assert vcs["git"]["sha1"] == snapshot["head"]
    return {"name": name, "version": package["version"], "source": package["source"], "index": index, "distribution": distribution, "verifiedFiles": files, "vcs": vcs}
