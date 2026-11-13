#!/usr/bin/env python3
"""One real MCP call, with a retained request/response trace."""
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import time

case = Path(sys.argv[1]).resolve()
manifest = json.loads((case / "case.json").read_text())
request = {"name": sys.argv[2], "arguments": json.loads(sys.argv[3])}
env = os.environ.copy()
env.update(OPENWARRANT_NO_PROJECTS="1", OPENWARRANT_NO_UPDATE_CHECK="1")
proc = subprocess.Popen([manifest["war"], "--root", manifest["root"], "admin", "mcp"],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    text=True, env=env, cwd=manifest["root"])
def exchange(message):
    proc.stdin.write(json.dumps(message) + "\n")
    proc.stdin.flush()
    if "id" not in message:
        return None
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        if not select.select([proc.stdout], [], [], max(0, deadline-time.monotonic()))[0]:
            raise TimeoutError("MCP response deadline")
        line = proc.stdout.readline(1048577)
        if not line or len(line) > 1048576:
            raise RuntimeError("missing or oversized MCP response")
        reply = json.loads(line)
        if reply.get("id") == message["id"]:
            return reply
    raise TimeoutError("MCP response deadline")
try:
    init = exchange({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "protocolVersion":"2025-06-18","capabilities":{},
        "clientInfo":{"name":"fresh-agent-workflow-evaluation","version":"1"}}})
    exchange({"jsonrpc":"2.0","method":"notifications/initialized"})
    reply = exchange({"jsonrpc":"2.0","id":2,"method":"tools/call","params":request})
    with (case / "calls.jsonl").open("a") as out:
        out.write(json.dumps({"request":request,"reply":reply}) + "\n")
    print(json.dumps(reply))
finally:
    if proc.stdin:
        proc.stdin.close()
        proc.stdin = None
    try:
        proc.communicate(timeout=3)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.communicate(timeout=3)
