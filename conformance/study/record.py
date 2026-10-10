#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Capture provisional OW95 timing; operator claims never establish qualification."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import signal
import sys
import tempfile
import time

from evaluate import CATEGORIES, PARTICIPANTS, unique_object

MAX_LINE = 4096
MAX_EVENTS = 1024
METHOD = "Python monotonic_ns sampled when commands are processed; categories, consent and outcomes are unverified operator claims"


def metadata(value):
    required = {"participant", "build_commit", "inputs_sha256", "scenario", "consent_confirmed", "assisted", "failures"}
    if not isinstance(value, dict) or not required <= value.keys() or value.keys() - required - {"retest_of"}:
        raise ValueError("metadata requires only the documented redacted fields")
    if value["participant"] not in PARTICIPANTS or value["consent_confirmed"] is not True:
        raise ValueError("a pseudonymous participant and operator-confirmed consent are required; this does not authenticate consent")
    for name, size in (("build_commit", 40), ("inputs_sha256", 64)):
        text = value[name]
        if not isinstance(text, str) or len(text) != size or any(c not in "0123456789abcdef" for c in text):
            raise ValueError("invalid declared build/input identity")
    if not isinstance(value["scenario"], str) or not 0 < len(value["scenario"].strip()) <= 2000:
        raise ValueError("a short redacted scenario is required")
    if type(value["assisted"]) is not bool or not isinstance(value["failures"], list) or len(value["failures"]) > 100:
        raise ValueError("assistance and failure declarations must be explicit")
    if any(not isinstance(v, str) or not 0 < len(v.strip()) <= 2000 for v in value["failures"]):
        raise ValueError("failures must be short redacted strings")
    if "retest_of" in value and (not isinstance(value["retest_of"], str) or not 0 < len(value["retest_of"].strip()) <= 256):
        raise ValueError("retest_of must identify the retained prior session")
    return {**value, "failures": list(value["failures"])}


class Recorder:
    def __init__(self, declared, clock=time.monotonic_ns, sink=lambda event: None):
        self.declared = metadata(declared)
        self.clock, self.sink = clock, sink
        self.origin = clock()
        if type(self.origin) is not int or self.origin < 0:
            raise ValueError("monotonic clock unavailable")
        self.offset = 0
        self.category, self.reason = "setup", None
        self.intervals, self.events = [], []
        self.outcome, self.termination = None, None
        self.timing_error = None
        self._event("started")

    def _event(self, kind, **extra):
        if len(self.events) >= MAX_EVENTS:
            raise ValueError("event limit reached")
        event = {"ordinal": len(self.events), "offset_ns": self.offset, "kind": kind, **extra}
        self.sink(event)
        self.events.append(event)

    def _sample(self):
        try:
            now = self.clock()
        except OSError as error:
            raise ValueError("monotonic clock unavailable") from error
        if type(now) is not int or now < self.origin + self.offset:
            raise ValueError("monotonic clock moved backwards or became unavailable")
        stop = now - self.origin
        if stop > self.offset:
            interval = {"start": self.offset / 1e9, "end": stop / 1e9, "category": self.category}
            if self.reason is not None:
                interval["reason"] = self.reason
            self.intervals.append(interval)
        self.offset = stop

    def command(self, value):
        if len(self.events) >= MAX_EVENTS - 1:
            raise ValueError("event limit reached; stop slot retained")
        if self.outcome is not None or not isinstance(value, dict):
            raise ValueError("command invalid or recorder already stopped")
        if "category" in value:
            category = value["category"]
            allowed = {"category", "reason"} if category == "excluded" else {"category"}
            if value.keys() != allowed or category not in CATEGORIES:
                raise ValueError("invalid category command")
            reason = value.get("reason")
            if category == "excluded" and (not isinstance(reason, str) or not 0 < len(reason.strip()) <= 2000):
                raise ValueError("exclusion needs a short redacted reason")
            self._sample()
            self.category, self.reason = category, reason
            self._event("category", category=category, **({"reason": reason} if reason else {}))
        elif value.keys() == {"assisted"} and value["assisted"] is True:
            self._sample()
            self.declared["assisted"] = True  # cannot erase a recorded intervention
            self._event("assistance")
        elif value.keys() == {"failure"} and isinstance(value["failure"], str) and 0 < len(value["failure"].strip()) <= 2000:
            if len(self.declared["failures"]) >= 100:
                raise ValueError("failure declaration limit reached")
            self._sample()
            self.declared["failures"].append(value["failure"])
            self._event("failure", reason=value["failure"])
        elif value.keys() == {"finish"} and value["finish"] in ("completed", "failed", "unknown"):
            self.stop("operator-finish", value["finish"])
        else:
            raise ValueError("unknown command; no arbitrary record fields are accepted")

    def stop(self, reason, outcome="unknown"):
        if self.outcome is not None:
            return
        try:
            self._sample()
        except (OSError, ValueError):
            self.timing_error = "monotonic clock observation unavailable"
            outcome = "unknown"
        self._event("stopped", termination=reason, outcome=outcome)
        self.outcome, self.termination = outcome, reason

    def record(self):
        if self.outcome is None:
            raise ValueError("recorder has not stopped")
        return {**self.declared, "schema": "oh.war/study-session-draft/v1",
                "host_os": os.uname().sysname, "measurement_method": METHOD,
                "outcome": self.outcome, "termination": self.termination,
                "elapsed_seconds": self.offset / 1e9 if self.offset and not self.timing_error else None,
                "timing_error": self.timing_error,
                "intervals": self.intervals, "qualification_established": False,
                "operator_claims_authenticated": False}


def commands(stream, seconds):
    deadline = time.monotonic() + seconds
    pending = b""
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0 or not select.select([stream], [], [], remaining)[0]:
            raise TimeoutError("session deadline reached")
        chunk = os.read(stream.fileno(), MAX_LINE + 1)
        if not chunk:
            if pending.strip():
                raise ValueError("incomplete command at end of input")
            return
        pending += chunk
        while b"\n" in pending:
            line, pending = pending.split(b"\n", 1)
            if len(line) > MAX_LINE:
                raise ValueError("command exceeds resource bound")
            if line.strip():
                yield json.loads(line, object_pairs_hook=unique_object)
        if len(pending) > MAX_LINE:
            raise ValueError("command exceeds resource bound")


def publish(path, record):
    # Same retention rule as telemetry: a new artifact must never replace one.
    raw = (json.dumps(record, indent=2, allow_nan=False) + "\n").encode()
    name = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=".study-publish-", delete=False) as stream:
            name = Path(stream.name)
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        os.link(name, path)
    finally:
        if name is not None:
            name.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--metadata", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--max-seconds", type=float, default=7200)
    args = parser.parse_args()
    recorder = None
    try:
        if not 0 < args.max_seconds <= 7200:
            raise ValueError("session bound must be positive and at most two hours")
        with args.metadata.open("rb") as stream:
            raw = stream.read(65_537)
        if len(raw) > 65_536:
            raise ValueError("metadata exceeds resource bound")
        declared = metadata(json.loads(raw, object_pairs_hook=unique_object))
        # Neither existing output nor an orphan event trail may be replaced.
        events = args.out.with_name(args.out.name + ".events.jsonl")
        if args.out.exists() or args.out.is_symlink():
            raise ValueError("output already exists; use a new session path")
        digest = hashlib.sha256()
        args.out.parent.mkdir(parents=True, exist_ok=True)
        flags = os.O_CREAT | os.O_EXCL | os.O_WRONLY | getattr(os, "O_NOFOLLOW", 0)
        with os.fdopen(os.open(events, flags, 0o600), "wb") as journal:
            def append(event):
                line = (json.dumps(event, separators=(",", ":"), allow_nan=False) + "\n").encode()
                journal.write(line)
                journal.flush()
                os.fsync(journal.fileno())
                digest.update(line)
            recorder = Recorder(declared, sink=append)
            def interrupted(*_):
                raise KeyboardInterrupt()
            previous = signal.signal(signal.SIGTERM, interrupted)
            try:
                for value in commands(sys.stdin, args.max_seconds):
                    recorder.command(value)
                    if recorder.outcome is not None:
                        break
                recorder.stop("end-of-input")
            except KeyboardInterrupt:
                recorder.stop("signal-interruption")
            except TimeoutError:
                recorder.stop("session-deadline")
            except (ValueError, RecursionError):
                recorder.stop("invalid-input")
            finally:
                signal.signal(signal.SIGTERM, previous)
            record = recorder.record()
            record["events"] = {"file": events.name, "sha256": digest.hexdigest(), "count": len(recorder.events)}
            record["metadata_sha256"] = hashlib.sha256(raw).hexdigest()
            publish(args.out, record)
        print(json.dumps({"session": str(args.out), "events": str(events), "outcome": record["outcome"], "qualification_established": False}))
        return 0 if record["outcome"] == "completed" else 1
    except (OSError, ValueError) as error:
        print(json.dumps({"error": str(error), "qualification_established": False}))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
