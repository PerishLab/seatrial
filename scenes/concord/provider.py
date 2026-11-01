import hashlib
import json
import os
from pathlib import Path
import sys


def connection(nodes):
    return {"nodes": nodes, "total": len(nodes), "page": {"next": False, "cursor": None}}


def render(marker):
    return "<!-- concord.acceptance/v1\n" + json.dumps(marker, separators=(",", ":")) + "\n-->"


def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()


def world():
    body = "## Problem\nObserver fixture\n## Outcome\nRetain unknown\n## Acceptance\n- [x] Fixture reviewed\n## Non-goals\nNo provider writes\n"
    declaration = render({"purpose": "declaration", "target": "release", "promise": "Observer fixture; absent distribution remains unknown"})
    closure = render({"purpose": "closure", "target": "release", "declaration": {"node": "IC_decl", "digest": digest(declaration)}, "judgment": "satisfied", "verification": "manual", "review": digest(body), "evidence": ["negative fixture only"], "remaining": [], "release": {"marker": "v0.0.0", "distribution": "https://releases.concord.perish.uk/v1/releases/stable/v0.0.0/distribution.json", "inclusion": ["adversarial review cannot prove an absent record"]}})
    comments = [{"id": name, "body": text, "url": f"https://github.com/Example/probe/issues/83#issuecomment-{index}"} for index, (name, text) in enumerate([("IC_decl", declaration), ("IC_close", closure)], 1)]
    issue = {"id": "I_probe", "number": 83, "url": "https://github.com/Example/probe/issues/83", "state": "OPEN", "kind": {"name": "Feature"}, "body": body, "updated": "2026-09-28T00:00:00Z", "labels": connection([{"name": "acceptance:release"}]), "comments": connection(comments), "children": connection([]), "blockers": connection([]), "edited": None, "events": connection([])}
    data = {"repository": {"id": "R_probe", "issue": issue}, "declaration": {"id": "IC_decl", "body": declaration, "created": "2026-09-29T00:00:00Z", "edited": None}, "closure": {"id": "IC_close", "body": closure, "created": "2026-09-30T00:00:00Z", "edited": None}}
    return data


def query(args):
    if "--input" in args:
        return json.loads(sys.stdin.read(1048576))["query"]
    return next(value.removeprefix("query=") for value in args if value.startswith("query="))


def respond(args, fixture):
    assert args and args[0] == "api", "provider permits only API reads"
    assert "--method" not in args and "-X" not in args, "provider method override refused"
    if fixture["mode"] == "context":
        assert args[1] == "repos/Example/probe" or (args[1] == "graphql" and query(args).lstrip().startswith("query")), "unexpected repository operation"
        return {"node": "R_probe", "coordinate": "Example/probe", "branch": "main"}
    assert args[1] == "graphql" and "--input" in args, "unexpected provider operation"
    text = query(args)
    assert text.lstrip().startswith("query("), "provider mutation refused"
    assert fixture["mode"] == "release"
    if 'object(expression:' in text:
        text = '[release]\nauthority = "https://releases.concord.perish.uk"\n'
        oid = hashlib.sha1(f"blob {len(text.encode())}\0".encode() + text.encode()).hexdigest()
        return {"data": {"repository": {"id": "R_probe", "object": {"oid": oid, "text": text}}}}
    assert any(word in text for word in ["labels(first:", "children:subIssues(", "declaration:node("]), "unexpected query shape"
    return {"data": world()}


def main():
    path = Path(os.environ["SEATRIAL_CONCORD_FIXTURE"])
    fixture = json.loads(path.read_text())
    reply = respond(sys.argv[1:], fixture)
    with path.with_suffix(".calls").open("a") as log:
        log.write("read\n")
    print(json.dumps(reply))


if __name__ == "__main__":
    main()
