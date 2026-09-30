#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Evaluate redacted OW95 timing records; never establish qualification."""
import argparse
import hashlib
import json
import math
from pathlib import Path

CATEGORIES = ("setup", "administration", "review", "implementation", "waiting", "excluded")
PARTICIPANTS = ("P1", "P2", "P3")


def number(value):
    return type(value) in (int, float) and 0 <= value <= 2**53 and math.isfinite(value)


def evaluate(records):
    results = []
    seen = set()
    for record in records:
        participant = record.get("participant")
        if participant not in PARTICIPANTS or participant in seen:
            raise ValueError("participant must be a unique P1, P2 or P3")
        seen.add(participant)
        if record.get("schema") != "oh.war/study-session-draft/v1":
            raise ValueError(f"{participant}: unsupported session schema")
        for field, size in (("build_commit", 40), ("inputs_sha256", 64)):
            value = record.get(field)
            if not isinstance(value, str) or len(value) != size or any(c not in "0123456789abcdef" for c in value):
                raise ValueError(f"{participant}: invalid {field}")
        for field in ("host_os", "scenario", "measurement_method"):
            if not isinstance(record.get(field), str) or not record[field].strip():
                raise ValueError(f"{participant}: missing {field}")
        if record.get("consent_confirmed") is not True:
            raise ValueError(f"{participant}: consent not confirmed")
        if type(record.get("assisted")) is not bool:
            raise ValueError(f"{participant}: assisted must be explicit")
        if not isinstance(record.get("failures"), list) or any(not isinstance(x, str) or not x.strip() for x in record["failures"]):
            raise ValueError(f"{participant}: failures must be an explicit list")
        if record.get("outcome") not in ("completed", "failed", "unknown"):
            raise ValueError(f"{participant}: invalid outcome")
        elapsed = record.get("elapsed_seconds")
        if elapsed is None:
            results.append({"participant": participant, "result": "unknown", "reason": "timing unavailable"})
            continue
        if not number(elapsed) or elapsed == 0:
            raise ValueError(f"{participant}: elapsed_seconds must be positive and finite")
        intervals = record.get("intervals")
        if not isinstance(intervals, list) or not intervals:
            raise ValueError(f"{participant}: intervals required")
        totals = {category: 0 for category in CATEGORIES}
        end = 0
        for interval in intervals:
            if not isinstance(interval, dict):
                raise ValueError(f"{participant}: interval must be an object")
            start, stop = interval.get("start"), interval.get("end")
            category = interval.get("category")
            if not number(start) or not number(stop) or start != end or stop <= start or stop > elapsed:
                raise ValueError(f"{participant}: intervals must cover elapsed time without gaps or overlap")
            if category not in CATEGORIES:
                raise ValueError(f"{participant}: unknown timing category")
            if category == "excluded" and (not isinstance(interval.get("reason"), str) or not interval["reason"].strip()):
                raise ValueError(f"{participant}: exclusion requires a reason")
            totals[category] += stop - start
            end = stop
        if end != elapsed:
            raise ValueError(f"{participant}: unaccounted elapsed time")
        thresholds = totals["setup"] <= 600 and totals["administration"] <= 60
        result = "within_thresholds" if thresholds and record["outcome"] == "completed" and not record["failures"] else "not_met"
        if record["outcome"] == "unknown":
            result = "unknown"
        if record["assisted"] and result == "within_thresholds":
            result = "review_required"
        results.append({"participant": participant, "result": result, "seconds": totals,
                        "elapsed_seconds": elapsed, "assisted": record["assisted"], "failures": record["failures"],
                        "build_commit": record["build_commit"], "inputs_sha256": record["inputs_sha256"]})
    missing = sorted(set(PARTICIPANTS) - seen)
    return {"schema": "oh.war/study-evaluation-draft/v1", "required_participants": 3,
            "observed_participants": len(results), "missing_participants": missing,
            "results": results, "all_measurements_within_thresholds": not missing and all(r["result"] == "within_thresholds" for r in results),
            "independent_review_required": True, "qualification_established": False}


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("records", nargs="*", type=Path)
    args = parser.parse_args()
    try:
        if len(args.records) > 3:
            raise ValueError("one selected session per participant; at most three records")
        records, sources = [], []
        for path in args.records:
            with path.open("rb") as stream:
                raw = stream.read(1_048_577)
            if len(raw) > 1_048_576:
                raise ValueError("record exceeds 1 MiB")
            record = json.loads(raw, object_pairs_hook=unique_object)
            if not isinstance(record, dict):
                raise ValueError("session must be an object")
            records.append(record)
            sources.append({"file": str(path), "sha256": hashlib.sha256(raw).hexdigest()})
        result = evaluate(records)
        result["sources"] = sources
        print(json.dumps(result, indent=2, allow_nan=False))
        return 0 if result["all_measurements_within_thresholds"] else 1
    except (OSError, ValueError) as error:
        print(json.dumps({"error": str(error), "qualification_established": False}))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
