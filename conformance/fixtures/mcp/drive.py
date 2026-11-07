#!/usr/bin/env python3
"""Feed a JSON-RPC transcript to `war mcp` one line at a time, waiting for the
answer to each request before sending the next, then close stdin. A pipe fed
straight from a file reaches EOF before the server has answered, and the
transport shuts down with requests in flight; a plant must not race that.
Usage: drive.py <war-binary> <transcript.jsonl> [war mcp args...]  — prints every
server line. `--all` serves the whole tool table; `--json` (the driver's, not
the server's) asks every tools/call for the oh.war/report/v1 envelope."""
import json
import subprocess
import sys

war, transcript, extra = sys.argv[1], sys.argv[2], sys.argv[3:]
envelope = "--json" in extra
extra = [a for a in extra if a != "--json"]
proc = subprocess.Popen([war, "mcp", *extra], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
for line in open(transcript, encoding="utf-8"):
    line = line.strip()
    if not line:
        continue
    msg = json.loads(line)
    if envelope and msg.get("method") == "tools/call":
        msg["params"].setdefault("arguments", {})["json"] = True
        line = json.dumps(msg)
    try:
        proc.stdin.write(line + "\n")
        proc.stdin.flush()
    except BrokenPipeError:
        sys.stderr.write("drive.py: the server closed its stdin before the transcript ended\n")
        break
    if "id" in msg:
        answer = proc.stdout.readline()
        if not answer:
            sys.stderr.write("drive.py: no answer for " + line[:80] + "\n")
            break
        sys.stdout.write(answer)
proc.stdin.close()
sys.exit(proc.wait(timeout=30))
