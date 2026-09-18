#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Local reference workflow. The SDK CLI owns document semantics; this app owns storage/UI."""

import argparse
import fcntl
import hashlib
import hmac
import json
import os
import re
import secrets
import socket
import subprocess
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from socketserver import ThreadingMixIn

from execution import ExecutionError, Executor
from drafting import Drafter
from hotline import Answers, HotlineError, digest as hotline_digest
from advice import Adviser
from verifier_service import Verification

BODY_LIMIT = 64 * 1024
FILE_LIMIT = 1024 * 1024
STORE_LIMIT = 64 * 1024 * 1024
REVISION_LIMIT = 2048
FIELDS = {"id", "title", "outcome", "scope", "context"}
HERE = Path(__file__).resolve().parent


class Refusal(Exception):
    def __init__(self, status, message):
        self.status = status
        self.message = message


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise Refusal(400, "Duplicate JSON field")
        result[key] = value
    return result


def decode(data):
    try:
        value = json.loads(data, object_pairs_hook=unique)
        # Escaped lone surrogates are accepted by Python's JSON decoder but cannot
        # be retained or served as UTF-8. Reject them at the shared input boundary.
        json.dumps(value, ensure_ascii=False).encode("utf-8")
        return value
    except (ValueError, UnicodeError, RecursionError) as e:
        raise Refusal(400, "Invalid JSON") from e


def digest(data):
    return hashlib.sha256(data).hexdigest()


def identity(value):
    if not isinstance(value, str) or not re.fullmatch(
        r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", value
    ):
        raise Refusal(400, "Expected a lowercase UUID")
    return value


def read_file(path, limit=FILE_LIMIT):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, "rb") as file:
        import stat

        if not stat.S_ISREG(os.fstat(file.fileno()).st_mode):
            raise Refusal(409, "Regular file required")
        data = file.read(limit + 1)
    if len(data) > limit:
        raise Refusal(413, "File limit exceeded")
    return data


def publish(path, data):
    fd, name = tempfile.mkstemp(prefix=".pending-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as file:
            file.write(data)
            file.flush()
            os.fsync(file.fileno())
        os.link(name, path)
        directory = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        os.unlink(name)


class SDK:
    def __init__(self, war, repo):
        self.war = war
        self.repo = repo

    def run(self, request, overview=False, board=False):
        args = (
            [str(self.war), "overview", "--snapshot", "--json"]
            if overview
            else [str(self.war), "sdk", "--request", "-"]
        )
        if board:
            args = [str(self.war), "board", "--json"]
        with tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
            try:
                result = subprocess.run(
                    check=False,
                    args=args,
                    input=None if overview or board else json.dumps(request).encode(),
                    stdout=out,
                    stderr=err,
                    cwd=self.repo,
                    timeout=10,
                )
            except (OSError, subprocess.TimeoutExpired) as e:
                raise Refusal(503, "SDK unavailable or timed out") from e
            if out.tell() > 4 * FILE_LIMIT:
                raise Refusal(503, "SDK output limit exceeded")
            out.seek(0)
            body = decode(out.read())
        if (
            result.returncode
            or not isinstance(body, dict)
            or body.get("exit_code") != 0
        ):
            raise Refusal(422, "SDK refused input")
        return body["result"]

    def author(self, fields, revision):
        title = fields["title"]
        result = self.run(
            {
                "schema": "oh.war/sdk-request/v1",
                "operation": "author",
                "metadata": [
                    ["schema", "oh.war/document/1.0.0-rc.3"],
                    ["kind", "warrant"],
                    ["id", "workflow:" + fields["id"]],
                    ["revision", revision],
                    ["title", title],
                    ["state", "draft"],
                ],
                "units": [
                    {
                        "id": key,
                        "kind": "binding",
                        "text": (
                            "# " + title if key == "outcome" else "## " + key.title()
                        )
                        + "\n\n"
                        + fields[key]
                        + "\n",
                    }
                    for key in ["outcome", "scope", "context"]
                ],
            }
        )
        if result.get("qualified") is not False or not isinstance(
            result.get("source"), str
        ):
            raise Refusal(503, "Unexpected SDK response")
        return result["source"]


def project_inventory(sdk):
    # Reuse the canonical viewer validator, including configured Warrant paths,
    # report types, evidence paths and invalid/unknown status handling.
    snapshot = sdk.run(None, overview=True)
    if snapshot.get("schema") != "oh.war/progress-view/v1":
        raise Refusal(503, "SDK does not support validated progress snapshots")
    rows = []
    for row in snapshot["legacy"]["warrants"]:
        entry = snapshot["reports"].get(row["alias"], {})
        report = entry.get("report")
        state = report["work_state"] if report else "not-reported"
        if entry.get("error"):
            route = "inspect-invalid-work-report"
        elif row["phase"] == "resolved":
            route = "preserve-resolved-history"
        elif state == "completed":
            route = "review-completion-evidence"
        elif row["assessment"] == "ready_to_resolve":
            route = "human-legacy-closure-or-explicit-reconciliation"
        elif state == "blocked":
            route = "complete-blocked-scope"
        else:
            route = "inspect-and-complete-scope"
        rows.append(
            {
                "alias": row["alias"],
                "title": row["title"],
                "legacy_phase": row["phase"],
                "legacy_assessment": row["assessment"],
                "work_state": state,
                "report_error": entry.get("error"),
                "summary": report.get("summary", "") if report else "",
                "route": route,
                "reconciled": False,
                "qualification_assessed": False,
            }
        )
    return {
        "schema": "oh.war/reconciliation-inventory/v1",
        "total": len(rows),
        "input_digest": snapshot["record_digest"],
        "source_revision": snapshot["source_revision"],
        "work_report_inputs": [
            {"path": r["source"], "sha256": r["source_digest"]}
            for r in snapshot["reports"].values()
        ],
        "all_remaining_reconciled": False,
        "qualification_assessed": False,
        "note": "Routes are proposals. Current file observations are not an atomic Git snapshot or acceptance.",
        "warrants": rows,
    }


class Store:
    def __init__(self, path, sdk):
        if path.is_symlink():
            raise Refusal(409, "State directory cannot be a symlink")
        path.mkdir(mode=0o700, parents=True, exist_ok=True)
        if path.stat().st_uid != os.getuid() or path.stat().st_mode & 0o077:
            raise Refusal(409, "State directory must be owner-only (0700)")
        self.path = path.resolve()
        self.sdk = sdk
        self.mutex = threading.Lock()
        self.lock_fd = os.open(
            self.path / ".lock", os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600
        )
        try:
            fcntl.flock(self.lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as e:
            raise Refusal(409, "State directory already in use") from e

    def records(self):
        records = []
        total = 0
        entries = list(self.path.iterdir())
        if len(entries) > REVISION_LIMIT + 64:
            raise Refusal(413, "Store entry limit exceeded")
        for path in sorted(entries):
            if path.name in (".lock", ".execution", ".drafting") or path.name.startswith(
                ".pending-"
            ):
                continue
            match = re.fullmatch(r"([0-9a-f-]{36})\.([0-9]{8})\.json", path.name)
            if not match:
                raise Refusal(409, "Unknown file in state directory")
            data = read_file(path)
            total += len(data)
            if total > STORE_LIMIT or len(records) >= REVISION_LIMIT:
                raise Refusal(413, "Store limit exceeded")
            envelope = decode(data)
            if (
                not isinstance(envelope, dict)
                or set(envelope) != {"schema", "payload", "sha256"}
                or envelope["schema"] != "oh.war/workflow-store/v1"
                or not isinstance(envelope["payload"], str)
                or digest(envelope["payload"].encode()) != envelope["sha256"]
            ):
                raise Refusal(409, "Stored record integrity mismatch")
            record = decode(envelope["payload"])
            keys = FIELDS | {
                "schema",
                "revision",
                "source",
                "source_sha256",
                "work_state",
                "qualified",
            }
            if (
                not isinstance(record, dict)
                or set(record) != keys
                or record.get("schema") != "oh.war/workflow-draft/v1"
                or type(record.get("revision")) is not int
                or any(
                    not isinstance(record.get(k), str)
                    or not record[k].strip()
                    or len(record[k].encode()) > 16000
                    for k in FIELDS
                )
            ):
                raise Refusal(409, "Invalid stored record shape")
            if (
                not isinstance(record, dict)
                or record.get("id") != match[1]
                or record.get("revision") != int(match[2])
                or record.get("qualified") is not False
                or record.get("work_state") != "draft"
            ):
                raise Refusal(409, "Invalid stored revision")
            if not isinstance(record.get("source"), str) or digest(
                record["source"].encode()
            ) != record.get("source_sha256"):
                raise Refusal(409, "Stored source digest mismatch")
            records.append(record)
        return records, total

    def get(self, id, revision=None):
        identity(id)
        with self.mutex:
            records, _ = self.records()
            matches = [
                r
                for r in records
                if r["id"] == id and (revision is None or r["revision"] == revision)
            ]
            if not matches:
                raise Refusal(404, "Unknown revision")
            return max(matches, key=lambda r: r["revision"])

    def listing(self):
        with self.mutex:
            records, _ = self.records()
            latest = {}
            for r in records:
                if r["id"] not in latest or r["revision"] > latest[r["id"]]["revision"]:
                    latest[r["id"]] = r
            return {
                "warrants": [
                    {
                        k: r[k]
                        for k in [
                            "id",
                            "title",
                            "revision",
                            "source_sha256",
                            "work_state",
                            "qualified",
                        ]
                    }
                    for r in latest.values()
                ]
            }

    def create(self, fields, expected=None):
        if not isinstance(fields, dict) or set(fields) != FIELDS:
            raise Refusal(400, "Expected id, title, outcome, scope and context only")
        identity(fields["id"])
        if any(
            not isinstance(v, str) or not v.strip() or len(v.encode()) > 16000
            for v in fields.values()
        ):
            raise Refusal(400, "Fields must contain bounded text")
        with self.mutex:
            records, total = self.records()
            previous = [r for r in records if r["id"] == fields["id"]]
            if expected is None and previous:
                raise Refusal(409, "Warrant already exists")
            if expected is not None and (
                not previous
                or max(previous, key=lambda r: r["revision"])["source_sha256"]
                != expected
            ):
                raise Refusal(409, "Revision changed; reload before editing")
            revision = max((r["revision"] for r in previous), default=0) + 1
            source = self.sdk.author(fields, revision)
            record = {
                **fields,
                "schema": "oh.war/workflow-draft/v1",
                "revision": revision,
                "source": source,
                "source_sha256": digest(source.encode()),
                "work_state": "draft",
                "qualified": False,
            }
            payload = json.dumps(record, ensure_ascii=False, indent=2) + "\n"
            # Hash exact stored payload bytes, not a protocol canonicalization.
            data = (
                json.dumps(
                    {
                        "schema": "oh.war/workflow-store/v1",
                        "payload": payload,
                        "sha256": digest(payload.encode()),
                    },
                    ensure_ascii=False,
                )
                + "\n"
            ).encode()
            if (
                len(data) > FILE_LIMIT
                or total + len(data) > STORE_LIMIT
                or len(records) >= REVISION_LIMIT
            ):
                raise Refusal(413, "Store limit exceeded")
            publish(self.path / (fields["id"] + f".{revision:08d}.json"), data)
            return record


class Server(ThreadingMixIn, HTTPServer):
    daemon_threads = True

    def __init__(self, address, store, token):
        self.store = store
        self.token = token
        self.executor = None
        self.drafter = None
        self.hotline = None
        self.verification = None
        self.slots = threading.BoundedSemaphore(8)
        super().__init__(address, Handler)

    def process_request(self, request, address):
        if not self.slots.acquire(blocking=False):
            request.close()
            return
        super().process_request(request, address)

    def process_request_thread(self, request, address):
        def expire():
            try:
                request.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass

        timer = threading.Timer(15, expire)
        timer.daemon = True
        timer.start()
        try:
            super().process_request_thread(request, address)
        finally:
            timer.cancel()
            self.slots.release()

    def handle_error(self, request, address):
        pass


class Handler(BaseHTTPRequestHandler):
    server_version = "OpenWarrantReference/0.1"

    def log_message(self, *args):
        pass

    def setup(self):
        super().setup()
        self.connection.settimeout(3)

    def reply(self, status, body, kind="application/json"):
        data = (
            json.dumps(body, ensure_ascii=False).encode()
            if kind == "application/json"
            else body
        )
        self.send_response(status)
        self.send_header("Content-Type", kind)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("Connection", "close")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header(
            "Content-Security-Policy",
            "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'",
        )
        self.end_headers()
        self.wfile.write(data)
        self.close_connection = True

    def dispatch(self):
        try:
            host = "127.0.0.1:" + str(self.server.server_port)
            if (
                self.headers.get_all("Host") != [host]
                or len(self.headers.get_all("Origin", [])) > 1
                or any(
                    o != "http://" + host for o in self.headers.get_all("Origin", [])
                )
            ):
                raise Refusal(403, "Local origin required")
            if self.command == "GET" and self.path == "/":
                return self.reply(
                    200, (HERE / "index.html").read_bytes(), "text/html; charset=utf-8"
                )
            auth = self.headers.get_all("Authorization", [])
            if len(auth) != 1 or not hmac.compare_digest(
                auth[0], "Bearer " + self.server.token
            ):
                raise Refusal(401, "Unlock with this service session token")
            store = self.server.store
            verification = re.fullmatch(r"/api/verification/([0-9a-f-]{36})", self.path)
            repair_preview = re.fullmatch(r"/api/verification/([0-9a-f-]{36})/repair", self.path)
            if self.command == "GET" and repair_preview:
                if self.server.verification is None:
                    raise Refusal(409, "Verifier not configured")
                return self.reply(200, self.server.verification.repair_preview(repair_preview[1]))
            if self.command == "GET" and (self.path == "/api/verification" or verification):
                if self.server.verification is None:
                    raise Refusal(409, "Verifier not configured")
                return self.reply(200, self.server.verification.get(verification[1]) if verification
                                  else self.server.verification.listing())
            if self.command == "GET" and self.path == "/api/warrants":
                return self.reply(200, store.listing())
            if self.command == "GET" and (self.path == "/api/drafting" or re.fullmatch(r"/api/drafting/[0-9a-f-]{36}", self.path)):
                if self.server.drafter is None:
                    raise Refusal(409, "Drafting harness not configured")
                return self.reply(200, self.server.drafter.listing() if self.path == "/api/drafting"
                                  else self.server.drafter.get(self.path.rsplit("/", 1)[1]))
            checkpoint_review = re.fullmatch(r"/api/hotline/([0-9a-f-]{36})/checkpoint", self.path)
            if self.command == "GET" and checkpoint_review:
                if self.server.hotline is None:
                    raise Refusal(409, "Hotline responders not configured")
                review = self.server.executor.question_checkpoint(checkpoint_review[1], self.server.hotline)
                return self.reply(200, {**review, "checkpoint_sha256": hotline_digest(review),
                                       "reconfirmation": self.server.hotline.checkpoint_answer(checkpoint_review[1], review)})
            if self.command == "GET" and self.path == "/api/hotline":
                if self.server.hotline is None:
                    raise Refusal(409, "Hotline responders not configured")
                data = self.server.hotline.listing()
                data.update(self.server.executor.adviser.listing() if self.server.executor.adviser else {"advice": []})
                return self.reply(200, data)
            if self.command == "GET" and self.path == "/api/board":
                board = store.sdk.run(None, board=True)
                if board.get("schema") != "oh.war/board-draft/v1":
                    raise Refusal(503, "SDK does not support the read-only board")
                return self.reply(200, board)
            if self.command == "GET" and self.path == "/api/project":
                return self.reply(200, project_inventory(store.sdk))
            stages = re.fullmatch(r"/api/stages/([0-9a-f-]{36})", self.path)
            if self.command == "GET" and stages:
                if self.server.executor is None:
                    raise Refusal(409, "Execution harness not configured")
                return self.reply(200, self.server.executor.stage_listing(stages[1]))
            if self.command == "GET" and self.path.startswith("/api/runs"):
                if self.server.executor is None:
                    raise Refusal(409, "Execution harness not configured")
                report = re.fullmatch(r"/api/runs/([0-9a-f-]{36})/report", self.path)
                if report:
                    return self.reply(200, self.server.executor.report(
                        report[1], self.server.completion_word, self.server.report_detail))
                if self.path == "/api/runs":
                    return self.reply(200, self.server.executor.listing())
                return self.reply(
                    200, self.server.executor.get(self.path.removeprefix("/api/runs/"))
                )
            match = re.fullmatch(r"/api/warrants/([0-9a-f-]{36})", self.path)
            if self.command == "GET" and match:
                return self.reply(200, store.get(match[1]))
            history = re.fullmatch(
                r"/api/warrants/([0-9a-f-]{36})/revisions/([1-9][0-9]{0,7})", self.path
            )
            if self.command == "GET" and history:
                return self.reply(200, store.get(history[1], int(history[2])))
            hotline_resume = re.fullmatch(r"/api/hotline/([0-9a-f-]{36})/resume", self.path)
            hotline_reconfirm = re.fullmatch(r"/api/hotline/([0-9a-f-]{36})/reconfirm", self.path)
            hotline_answer = re.fullmatch(r"/api/hotline/([0-9a-f-]{36})/answer", self.path)
            verifier_start = re.fullmatch(r"/api/verification/([0-9a-f-]{36})/start", self.path)
            verifier_rebuttal = re.fullmatch(r"/api/verification/([0-9a-f-]{36})/rebuttal", self.path)
            if (
                self.command == "POST" and (hotline_answer or hotline_resume or hotline_reconfirm or verifier_start or repair_preview or verifier_rebuttal)
            ) or (
                self.command == "POST" and self.path in ("/api/warrants", "/api/runs", "/api/admission", "/api/drafting", "/api/verification")
            ) or (self.command == "PUT" and match):
                if (
                    self.headers.get_all("Content-Type") != ["application/json"]
                    or self.headers.get_all("Transfer-Encoding")
                    or self.headers.get_all("Expect")
                ):
                    raise Refusal(400, "JSON with Content-Length required")
                lengths = self.headers.get_all("Content-Length", [])
                if len(lengths) != 1 or not re.fullmatch(r"[0-9]{1,8}", lengths[0]):
                    raise Refusal(400, "One Content-Length required")
                size = int(lengths[0])
                if size > BODY_LIMIT:
                    raise Refusal(413, "Request body limit exceeded")
                body = self.rfile.read(size)
                if len(body) != size:
                    raise Refusal(400, "Incomplete request")
                fields = decode(body)
                if verifier_rebuttal:
                    if self.server.verification is None:
                        raise Refusal(409, "Verifier not configured")
                    return self.reply(200, self.server.verification.rebut(verifier_rebuttal[1], fields))
                if repair_preview:
                    if self.server.verification is None:
                        raise Refusal(409, "Verifier not configured")
                    return self.reply(202, self.server.verification.repair(repair_preview[1], fields))
                if verifier_start:
                    if self.server.verification is None:
                        raise Refusal(409, "Verifier not configured")
                    return self.reply(202, self.server.verification.start(verifier_start[1], fields))
                if self.path == "/api/verification":
                    if self.server.verification is None:
                        raise Refusal(409, "Verifier not configured")
                    return self.reply(200, self.server.verification.prepare(fields))
                if hotline_resume:
                    if self.server.hotline is None:
                        raise Refusal(409, "Hotline responders not configured")
                    return self.reply(202, self.server.executor.resume(hotline_resume[1], fields, self.server.hotline))
                if hotline_answer or hotline_reconfirm:
                    if self.server.hotline is None:
                        raise Refusal(409, "Hotline responders not configured")
                    credentials = self.headers.get_all("X-OW-Responder", [])
                    if len(credentials) != 1:
                        raise Refusal(401, "One responder credential required")
                    if hotline_reconfirm:
                        return self.reply(200, self.server.hotline.reconfirm(hotline_reconfirm[1], fields, credentials[0]))
                    return self.reply(200, self.server.hotline.submit(hotline_answer[1], fields, credentials[0]))
                if self.path == "/api/drafting":
                    if self.server.drafter is None:
                        raise Refusal(409, "Drafting harness not configured")
                    return self.reply(202, self.server.drafter.start(fields))
                if self.path in ("/api/runs", "/api/admission"):
                    if self.server.executor is None:
                        raise Refusal(409, "Execution harness not configured")
                    if self.path == "/api/admission":
                        return self.reply(200, self.server.executor.admission(fields))
                    return self.reply(202, self.server.executor.start(fields))
                expected = None
                if self.command == "PUT":
                    if not isinstance(fields, dict) or fields.get("id") != match[1]:
                        raise Refusal(400, "Request identity mismatch")
                    expected = fields.pop("expected_source_sha256", None)
                    if not isinstance(expected, str) or not re.fullmatch(
                        r"[0-9a-f]{64}", expected
                    ):
                        raise Refusal(400, "Expected previous source digest required")
                return self.reply(
                    201 if self.command == "POST" else 200,
                    store.create(fields, expected),
                )
            raise Refusal(404, "Unsupported route or action")
        except (Refusal, ExecutionError, HotlineError) as e:
            self.reply(e.status, {"error": e.message, "qualified": False})
        except (OSError, KeyError, TypeError, ValueError, subprocess.SubprocessError):
            self.reply(
                409,
                {
                    "error": "Stored data or service input unavailable; no successful write claimed",
                    "qualified": False,
                },
            )

    do_GET = dispatch
    do_POST = dispatch
    do_PUT = dispatch
    do_DELETE = dispatch


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--war", type=Path, required=True)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--state", type=Path, required=True)
    parser.add_argument("--port", type=int, default=8766)
    parser.add_argument("--session-file", type=Path, required=True)
    parser.add_argument("--execution-config", type=Path)
    parser.add_argument("--drafting-config", type=Path)
    parser.add_argument("--hotline-config", type=Path)
    parser.add_argument("--adviser-config", type=Path)
    parser.add_argument("--verifier-config", type=Path)
    parser.add_argument("--verifier-issuer", type=Path)
    parser.add_argument("--completion-word", default="WORK_DONE")
    parser.add_argument("--report-detail", choices=("minimal", "full"), default="full")
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z0-9_-]{1,64}", args.completion_word):
        parser.error("completion word must contain 1-64 letters, digits, underscore or hyphen")
    store = Store(
        args.state, SDK(args.war.resolve(strict=True), args.repo.resolve(strict=True))
    )
    token = secrets.token_hex(32)
    server = Server(("127.0.0.1", args.port), store, token)
    server.completion_word = args.completion_word
    server.report_detail = args.report_detail
    if args.drafting_config:
        server.drafter = Drafter(store, args.drafting_config, read_file, publish, decode)
    if args.execution_config:
        server.executor = Executor(
            store, args.execution_config, read_file, publish, decode
        )
    if args.hotline_config:
        if server.executor is None:
            parser.error("hotline requires execution configuration")
        server.hotline = Answers(server.executor, decode(read_file(args.hotline_config, 65536)))
    if args.verifier_config or args.verifier_issuer:
        if server.executor is None or not args.verifier_config or not args.verifier_issuer:
            parser.error("verifier requires execution configuration, verifier configuration and pinned issuer file")
        server.verification = Verification(server.executor, args.verifier_config, args.verifier_issuer)
    if args.adviser_config:
        if server.hotline is None:
            parser.error("adviser requires hotline configuration")
        server.executor.adviser = Adviser(server.hotline, decode(read_file(args.adviser_config, 65536)))
        server.executor.adviser.schedule()
    info = {"url": "http://127.0.0.1:" + str(server.server_port), "token": token}
    publish(args.session_file, (json.dumps(info) + "\n").encode())
    print(info["url"], flush=True)
    try:
        server.serve_forever()
    finally:
        server.server_close()
        os.close(store.lock_fd)


if __name__ == "__main__":
    main()
