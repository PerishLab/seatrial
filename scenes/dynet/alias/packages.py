import concurrent.futures
from pathlib import Path
import shlex
import subprocess
from urllib.parse import unquote, urlsplit

root = Path("/fixture/debian")
mirror = root / "mirror"
lists = root / "lists"
sources = root / "sources.list"
apt = ["apt-get", "-o", f"Dir::State::Lists={lists}", "-o", f"Dir::Etc::sourcelist={sources}",
       "-o", "Dir::Etc::sourceparts=-", "-o", "Acquire::Languages=none"]
upstreams = (("debian", "trixie"), ("debian", "trixie-updates"), ("debian-security", "trixie-security"))


def fetch(path):
    destination = mirror / unquote(path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    for host in ("deb.debian.org", "mirrors.ustc.edu.cn"):
        fetched = subprocess.run(["curl", "-fsSL", "--connect-timeout", "10", "--max-time", "45",
                                  "--output", str(destination), f"https://{host}/{path}"])
        if fetched.returncode == 0:
            return
    fetched.check_returncode()


def collect(paths):
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        list(pool.map(fetch, paths))


def packages(output):
    for line in output.splitlines():
        if not line.startswith("'file:"):
            continue
        path = Path(urlsplit(shlex.split(line)[0]).path).relative_to(mirror)
        if path.parts[:2] not in (("debian", "pool"), ("debian-security", "pool")) or ".." in Path(unquote(str(path))).parts:
            raise ValueError(f"unexpected package path {path}")
        yield path.as_posix()


arch = subprocess.check_output(["dpkg", "--print-architecture"], text=True).strip()
indexes = []
entries = []
for repo, suite in upstreams:
    prefix = f"{repo}/dists/{suite}"
    indexes.extend((f"{prefix}/InRelease", f"{prefix}/main/binary-{arch}/Packages.xz"))
    entries.append(f"deb [arch={arch} by-hash=no signed-by=/usr/share/keyrings/debian-archive-keyring.gpg] file:{mirror}/{repo} {suite} main")
collect(indexes)
lists.mkdir(exist_ok=True)
sources.write_text("\n".join(entries) + "\n", encoding="utf-8")
subprocess.run(apt + ["-o", "APT::Update::Error-Mode=any", "update"], check=True)
selection = subprocess.check_output(apt + ["--print-uris", "-y", "--no-install-recommends",
                                          "install", "iproute2", "nftables", "systemd-resolved"], text=True)
collect(set(packages(selection)))
