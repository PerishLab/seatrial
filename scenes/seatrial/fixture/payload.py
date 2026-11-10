import hashlib
import json
import sys
from pathlib import Path
import subprocess

path = Path(sys.argv[1])
with path.open("rb") as stream:
    data = stream.read(33554433)
assert len(data) <= 33554432
version = subprocess.run([str(path), "--version"], capture_output=True, text=True, check=True).stdout.strip()
print("PLACEMENT_ACTUAL=" + json.dumps({"sha256": hashlib.sha256(data).hexdigest(), "version": version}))
