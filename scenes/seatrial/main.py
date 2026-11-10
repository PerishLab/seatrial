import json
from pathlib import Path
import re
import sys
import tempfile
from types import SimpleNamespace

import reference
import trial


def main():
    assert len(sys.argv) == 6
    frame = SimpleNamespace(binary=sys.argv[1], lane=sys.argv[2], image=sys.argv[3], probe=sys.argv[4], fixture=Path(sys.argv[5]))
    assert re.fullmatch(r"st[0-9a-f]{8}", frame.lane)
    with tempfile.TemporaryDirectory(prefix="seatrial-placement-") as directory:
        root = Path(directory)
        expected = reference.resolve(root, frame)
        positive = trial.run(root, frame, "positive")
        assert positive["exit"] == 0, positive
        record = positive["report"]
        assert record["ok"] and record["cleanup"] and record["state"] == "passed"
        assert len(record["products"]) == 1 and len(record["cases"]) == 1
        product = record["products"][0]
        for key in ["name", "channel", "version", "commit", "seal", "archive"]:
            assert product[key] == expected[key], (key, product, expected)
        assert product["placement"] == trial.placement()
        actual = positive["actual"]
        assert actual == expected["file"], (actual, expected)
        negative = trial.run(root, frame, "negative")
        refused = negative["report"]
        assert negative["exit"] == 2 and refused["cleanup"]
        assert not refused["cases"] and not refused["products"] and not refused["ok"]
        assert "canonical relative paths" in refused["error"], refused
        print(json.dumps({"assertion": "distributed runner prepares exact Debian service bytes through public placement; malformed path refuses; both nested lanes clean", "fixture": expected, "positive": record, "negative": refused}), flush=True)


if __name__ == "__main__":
    main()
