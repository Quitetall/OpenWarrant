# SPDX-License-Identifier: Apache-2.0
"""Append-only verifier dispatch claims. Protected storage remains a harness duty."""
import json
import re
from pathlib import Path

from hotline import digest
from verification import identity, request, request_digest, require, result


class Jobs:
    """Persist before launch; a consumed claim must never launch again after restart.

    publish must atomically create without replacement and fsync file and directory.
    read_file must refuse symlinks/nonregular files and enforce a size bound.
    This store does not authenticate admission or prove that old processes stopped.
    """

    def __init__(self, root, *, publish, read_file, decode):
        self.root = Path(root)
        require(not self.root.is_symlink(), "Verifier job directory cannot be a symlink")
        self.root.mkdir(mode=0o700, exist_ok=True)
        require(self.root.is_dir() and self.root.stat().st_mode & 0o077 == 0,
                "Private verifier job directory required")
        self.publish, self.read_file, self.decode = publish, read_file, decode

    def path(self, id, sequence):
        require(identity(id), "Exact verifier job identity required")
        return self.root / f"{id}.{sequence}.json"

    def save(self, record):
        data = json.dumps({"record": record, "sha256": digest(record)},
                          ensure_ascii=False, allow_nan=False).encode()
        require(len(data) <= 1024 * 1024, "Verifier job record too large")
        self.publish(self.path(record["verification_id"], record["sequence"]), data)

    def read(self, id):
        previous = None
        for sequence in (1, 2, 3):
            try:
                envelope = self.decode(self.read_file(self.path(id, sequence)))
            except FileNotFoundError:
                continue
            require(isinstance(envelope, dict) and set(envelope) == {"record", "sha256"}
                    and isinstance(envelope["record"], dict)
                    and envelope["sha256"] == digest(envelope["record"]), "Verifier job integrity mismatch")
            record = envelope["record"]
            common = {"schema", "verification_id", "sequence", "previous_sha256"}
            extra = {1: {"request", "basis_sha256"}, 2: {"protection_sha256"}, 3: {"observation"}}[sequence]
            require(set(record) == common | extra and record["schema"] == "oh.war/verifier-job/v1"
                    and record["verification_id"] == id and type(record["sequence"]) is int
                    and record["sequence"] == sequence, "Verifier job identity mismatch")
            require((sequence == 1 and previous is None and record["previous_sha256"] is None)
                    or (previous is not None and previous["sequence"] == sequence - 1
                        and record["previous_sha256"] == digest(previous)), "Verifier job history broken")
            if sequence == 1:
                expected = request(record["request"])
                require(expected["verification_id"] == id and self.hash(record["basis_sha256"]),
                        "Verifier claim basis mismatch")
            elif sequence == 2:
                require(self.hash(record["protection_sha256"]), "Exact protection receipt digest required")
            else:
                self.observation(record["observation"], expected)
            previous = record
        return previous

    @staticmethod
    def hash(value):
        return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None

    @staticmethod
    def observation(value, expected):
        require(isinstance(value, dict) and value.get("schema") == "oh.war/verifier-observation/v1"
                and value.get("request_sha256") == request_digest(expected)
                and value.get("qualified") is False and value.get("execution_state") in ("stopped", "unknown")
                and value.get("verdict") in ("pass", "fail", "unknown"), "Invalid verifier observation")
        require(value["execution_state"] == "stopped" or value["verdict"] == "unknown",
                "Uncertain execution cannot establish verdict")
        if value.get("result") is not None:
            parsed = result(value["result"], expected)
            require(parsed["verdict"] == value["verdict"], "Verifier verdict mismatch")
        if value["verdict"] == "pass":
            require(value.get("result") is not None, "Passing observation requires exact verifier result")
            checks = value.get("checks")
            require(isinstance(checks, list) and len(checks) == len(expected["checks"])
                    and all(isinstance(actual, dict) and actual.get("argv") == command
                            and type(actual.get("exit_code")) is int and actual["exit_code"] == 0
                            for actual, command in zip(checks, expected["checks"])),
                    "Passing observation requires every exact protected check")
        if value["verdict"] == "fail" and value.get("result") is None:
            checks = value.get("checks")
            require(isinstance(checks, list) and 0 < len(checks) <= len(expected["checks"])
                    and all(isinstance(actual, dict) and actual.get("argv") == command
                            and type(actual.get("exit_code")) is int
                            for actual, command in zip(checks, expected["checks"]))
                    and any(actual["exit_code"] != 0 for actual in checks),
                    "Failed observation requires a verifier finding or failed protected check")

    def record(self, id, sequence, prior, **fields):
        return {"schema": "oh.war/verifier-job/v1", "verification_id": id,
                "sequence": sequence, "previous_sha256": digest(prior) if prior else None, **fields}

    def claim(self, expected, basis_sha256):
        expected = request(expected)
        require(self.hash(basis_sha256), "Exact admission basis required")
        initial = self.record(expected["verification_id"], 1, None,
                              request=expected, basis_sha256=basis_sha256)
        try:
            self.save(initial)
            return True
        except FileExistsError:
            # Same id is an idempotency key, never permission to dispatch again.
            self.read(expected["verification_id"])
            stored = self.decode(self.read_file(self.path(expected["verification_id"], 1)))["record"]
            require(stored == initial, "Verifier identity already binds different work")
            return False

    def begin(self, id, protection_sha256):
        require(self.hash(protection_sha256), "Exact authenticated protection receipt digest required")
        prior = self.read(id)
        require(prior is not None, "Verifier claim missing")
        if prior["sequence"] != 1:
            return False
        try:
            self.save(self.record(id, 2, prior, protection_sha256=protection_sha256))
            return True
        except FileExistsError:
            self.read(id)
            return False

    def finish(self, id, observation):
        prior = self.read(id)
        require(prior is not None and prior["sequence"] == 2, "Consumed unfinished verifier claim required")
        initial = self.decode(self.read_file(self.path(id, 1)))["record"]
        self.observation(observation, initial["request"])
        self.save(self.record(id, 3, prior, observation=observation))

    def view(self, id, *, live=False):
        record = self.read(id)
        if record is None:
            return None
        state = {1: "prepared", 2: "running" if live else "unknown", 3: "finished"}[record["sequence"]]
        return {"verification_id": id, "state": state, "record": record, "qualified": False}
