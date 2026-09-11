#!/usr/bin/env python3
"""Feed a JSON-RPC transcript to `war mcp` one line at a time, waiting for the
answer to each request before sending the next, then close stdin. A pipe fed
straight from a file reaches EOF before the server has answered, and the
transport shuts down with requests in flight; a plant must not race that.
Usage: drive.py <war-binary> <transcript.jsonl>  — prints every server line."""
import json
import subprocess
import sys

war, transcript = sys.argv[1], sys.argv[2]
proc = subprocess.Popen([war, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
for line in open(transcript, encoding="utf-8"):
    line = line.strip()
    if not line:
        continue
    try:
        proc.stdin.write(line + "\n")
        proc.stdin.flush()
    except BrokenPipeError:
        sys.stderr.write("drive.py: the server closed its stdin before the transcript ended\n")
        break
    if "id" in json.loads(line):
        answer = proc.stdout.readline()
        if not answer:
            sys.stderr.write("drive.py: no answer for " + line[:80] + "\n")
            break
        sys.stdout.write(answer)
proc.stdin.close()
sys.exit(proc.wait(timeout=30))
