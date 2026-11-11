import copy
import json
from pathlib import Path
import sys


def page(nodes):
    return {"nodes": nodes, "totalCount": len(nodes), "pageInfo": {"hasNextPage": False, "endCursor": None}}


def mutation(state, request):
    names = [name for name in ["addProjectV2ItemById", "createProjectV2StatusUpdate"] if "change:" + name + "(" in request["query"]]
    assert len(names) == 1, "unexpected mutation"
    name = names[0]
    value = request["variables"]["input"]
    assert value["projectId"] == state["project"]["id"], "foreign project"
    state["writes"].append(name)
    if name == "addProjectV2ItemById":
        assert value["contentId"] == state["issue"]["id"], "foreign issue"
        state["items"].append({"id": "ITEM_trial", "type": "ISSUE", "isArchived": False, "updatedAt": "original", "project": {"id": state["project"]["id"]}, "content": state["issue"]})
        state["hidden"] = True
    else:
        state["statusUpdates"].append({"id": "U_" + str(len(state["statusUpdates"])), "body": value["body"], "createdAt": "original", "updatedAt": "original"})
    state["project"]["updatedAt"] = "stamp-" + str(len(state["writes"]))
    return {"data": {"change": {"clientMutationId": None}}}


def query(state, request):
    text = request["query"]
    values = request.get("variables", {})
    if "organization(login:" in text:
        assert values == {"owner": "Example", "number": 1}, "foreign target"
        return {"data": {"organization": {"projectV2": state["project"]}}}
    if "nodes(ids:" in text:
        assert values["ids"] == [state["issue"]["id"]], "foreign issue lookup"
        return {"data": {"nodes": [state["issue"]]}}
    assert values.get("id") == state["project"]["id"], "foreign project lookup"
    project = copy.deepcopy(state["project"])
    for name in ["fields", "items", "views", "workflows", "repositories", "statusUpdates"]:
        if name + "(first:" in text:
            nodes = state[name]
            if name == "items" and state["hidden"]:
                state["observations"] += 1
                nodes = []
            project[name] = page(nodes)
    return {"data": {"node": project}}


def respond(state, args):
    state["calls"] += 1
    if args == ["api", "--method", "GET", "--include", "orgs/Example/projectsV2/1/fields?per_page=100"]:
        return "HTTP/2.0 200 OK\r\nContent-Type: application/json\r\n\r\n[]"
    assert args == ["api", "graphql", "--input", "-"], "unexpected provider operation"
    request = json.load(sys.stdin)
    if request["query"].startswith("mutation("):
        return json.dumps(mutation(state, request))
    assert request["query"].startswith("query("), "unexpected query"
    return json.dumps(query(state, request))


def main():
    path = Path(sys.argv[1])
    state = json.loads(path.read_text())
    reply = respond(state, sys.argv[2:])
    path.write_text(json.dumps(state))
    print(reply, end="")


if __name__ == "__main__":
    main()
