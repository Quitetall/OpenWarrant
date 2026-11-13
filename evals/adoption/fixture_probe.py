#!/usr/bin/python3
"""A scripted session that tries to reach what no agent may: amendment 7's leak check.

It runs in a real agent's sandbox, with nothing mounted beyond what a real agent gets except this
script. It changes nothing, so a relay of it stalls after two sessions, and the second session is
the one that matters: by then the harness has graded the first and written its records.

It tries, and reports each attempt's outcome:
  - every path in STUDY_PROBE_PATHS (earlier sessions' grade.json under /work and under the cell's
    host path, the study's hidden/ and reference/ on the host, the keep store, the harness's code,
    and where the harness mounts hidden tests and fixtures inside a sandbox): stat, then list or read;
  - every file anywhere it can see whose name is in STUDY_PROBE_NAMES (hidden tests, reference
    patches, backlog.toml) or, outside /usr, is a session record (grade.json, tracker.json, ...);
  - any small file under /work, /tmp, /run, /home or /opt that holds a grade (its JSON keys).

Its stdout is a `probe` event, then one stream-json `result` line, so the metrics parser runs on it.
"""

import json
import os
from pathlib import Path

SESSION_RECORDS = ("grade.json", "tracker.json", "metrics.json", "result.json", "stream.jsonl",
                   "prompt.txt", "stderr.txt", "spec.json", "questions.jsonl")
SKIP_ALWAYS = ("/proc", "/sys", "/dev")
SELF = "/opt/study/fixture_probe.py"  # names a grade's keys itself
# Shared build caches and toolchains: large, and holding nothing of a cell's own (amendment 7).
SKIP_DIRS = ("/opt/cargo-home", "/opt/sccache", "/home/agent/.rustup", "/opt/study/py")


def attempt(path: str) -> dict:
    try:
        st = os.stat(path)
    except OSError as e:
        return {"path": path, "reachable": False, "error": f"{type(e).__name__}: {e.strerror}"}
    try:
        if os.path.isdir(path):
            got = f"listed {len(os.listdir(path))} entries"
        else:
            with open(path, "rb") as f:
                got = f"read {len(f.read(4096))} bytes"
    except OSError as e:
        return {"path": path, "reachable": False, "error": f"stat ok (mode {oct(st.st_mode)}), then {type(e).__name__}: {e.strerror}"}
    return {"path": path, "reachable": True, "got": got}


def walk(root: str):
    for d, subdirs, files in os.walk(root, onerror=lambda e: None):
        subdirs[:] = [s for s in subdirs
                      if f"{d.rstrip('/')}/{s}" not in SKIP_ALWAYS + SKIP_DIRS and s not in (".git", "target")]
        yield d, files


def main() -> None:
    paths = [p for p in os.environ.get("STUDY_PROBE_PATHS", "").split("\n") if p]
    names = set(n for n in os.environ.get("STUDY_PROBE_NAMES", "").split("\n") if n)
    attempts = [attempt(p) for p in paths]
    named, grades = [], []
    for d, files in walk("/"):
        in_usr = d == "/usr" or d.startswith("/usr/")
        for f in files:
            p = f"{d.rstrip('/')}/{f}"
            if f in names or (f in SESSION_RECORDS and not in_usr):
                named.append(p)
            if not in_usr and d.startswith(("/work", "/tmp", "/run", "/home", "/opt")) and p != SELF:
                try:
                    if os.path.getsize(p) < 1 << 20:
                        text = Path(p).read_text(errors="ignore")
                        if '"items"' in text and '"pass"' in text and '"tail"' in text:
                            grades.append(p)
                except OSError:
                    pass
    report = {"type": "probe", "cwd": os.getcwd(), "work_entries": sorted(os.listdir("/work")),
              "attempts": attempts, "reachable": [a["path"] for a in attempts if a["reachable"]],
              "forbidden_names_found": sorted(named)[:60], "grade_like_files": sorted(grades)[:20]}
    print(json.dumps(report))
    print(json.dumps({"type": "result", "subtype": "success", "is_error": False, "num_turns": 0, "duration_ms": 0,
                      "total_cost_usd": 0, "result": "probe: changed nothing",
                      "usage": {"input_tokens": 0, "output_tokens": 0, "cache_read_input_tokens": 0,
                                "cache_creation_input_tokens": 0}}))


if __name__ == "__main__":
    main()
