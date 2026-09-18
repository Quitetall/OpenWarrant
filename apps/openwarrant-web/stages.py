# SPDX-License-Identifier: Apache-2.0
"""Workflow stage planning. No document parsing, authority grants or execution."""
import re


class StageError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise StageError(message)


def validate(graph):
    """Explicit workflow dependencies, distinct from milestone acceptance records."""
    require(isinstance(graph, dict) and 1 <= len(graph) <= 64, "Expected 1-64 stages")
    require(all(isinstance(k, str) and re.fullmatch(r"[A-Za-z0-9_-]{1,128}", k)
                for k in graph), "Invalid stage identity")
    for id, dependencies in graph.items():
        require(isinstance(dependencies, list)
                and all(isinstance(d, str) and d in graph for d in dependencies)
                and len(dependencies) == len(set(dependencies)), "Unknown or duplicate stage dependency")
        require(id not in dependencies, "Stage cannot depend on itself")
    visiting, visited = set(), set()

    def visit(id):
        require(id not in visiting, "Stage dependency cycle")
        if id in visited:
            return
        visiting.add(id)
        for dep in graph[id]:
            visit(dep)
        visiting.remove(id)
        visited.add(id)

    for id in graph:
        visit(id)
    return {id: tuple(deps) for id, deps in graph.items()}


def frontier(graph, completed=(), question_blocks=(), writer=None):
    """Project current observations. Caller must recheck them when claiming writer."""
    graph = validate(graph)
    require(all(isinstance(items, (list, tuple, set, frozenset))
                and all(isinstance(item, str) for item in items)
                for items in (completed, question_blocks)), "Expected stage observation collections")
    done, seeds = set(completed), set(question_blocks)
    require(done <= graph.keys() and seeds <= graph.keys(), "Unknown stage observation")
    require(writer is None or writer in graph, "Unknown active writer stage")
    require(writer not in done, "Completed stage cannot retain an active writer")
    require(writer is None or set(graph[writer]) <= done, "Active stage has incomplete prerequisite")
    require(all(set(graph[id]) <= done for id in done), "Completed stage has incomplete prerequisite")
    blocked = set(seeds)
    while True:
        expanded = blocked | {id for id, deps in graph.items() if blocked.intersection(deps)}
        if expanded == blocked:
            break
        blocked = expanded
    require(not done.intersection(blocked), "Blocked work cannot also be current completion")
    rows = []
    for id in sorted(graph):
        waiting = sorted(set(graph[id]) - done)
        if id == writer:
            state = "stop-required" if id in blocked else "running"
        elif id in done:
            state = "completed"
        elif id in blocked:
            state = "blocked"
        elif waiting:
            state = "waiting-for-prerequisite"
        elif writer is not None:
            state = "waiting-for-writer"
        else:
            state = "ready"
        rows.append({"stage": id, "state": state, "waiting_on": waiting,
                     "question_blocked": id in blocked, "dispatch_permitted": False})
    return rows
