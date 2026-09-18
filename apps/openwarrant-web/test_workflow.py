# SPDX-License-Identifier: Apache-2.0
"""Public HTTP seam, using the real built OpenWarrant SDK CLI."""

import json
import os
import subprocess
import sys
import tempfile
import time
import unittest
import urllib.error
import urllib.request
import uuid
from pathlib import Path

APP = Path(__file__).with_name("server.py")
REPO = Path(__file__).resolve().parents[2]
WAR = os.environ.get("OW_TEST_WAR", str(REPO / "target/debug/war"))


class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.start()

    def start(self):
        self.session = self.root / ("session-" + str(uuid.uuid4()) + ".json")
        self.proc = subprocess.Popen(
            [
                sys.executable,
                str(APP),
                "--war",
                WAR,
                "--repo",
                str(REPO),
                "--state",
                str(self.root / "state"),
                "--port",
                "0",
                "--session-file",
                str(self.session),
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        for _ in range(200):
            if self.session.exists():
                break
            if self.proc.poll() is not None:
                message = self.proc.stderr.read().decode()
                self.proc.stderr.close()
                self.tmp.cleanup()
                self.fail(message)
            time.sleep(0.02)
        self.assertTrue(self.session.exists(), "server startup")
        info = json.loads(self.session.read_text())
        self.url = info["url"]
        self.token = info["token"]

    def stop(self):
        self.proc.terminate()
        self.proc.wait(timeout=10)
        self.proc.stderr.close()

    def tearDown(self):
        if self.proc.poll() is None:
            self.stop()
        self.tmp.cleanup()

    def call(self, path, method="GET", data=None, headers=None):
        h = {"Authorization": "Bearer " + self.token}
        if data is not None:
            h["Content-Type"] = "application/json"
        h.update(headers or {})
        req = urllib.request.Request(
            self.url + path,
            data=None if data is None else json.dumps(data).encode(),
            headers=h,
            method=method,
        )
        try:
            r = urllib.request.urlopen(req, timeout=15)
        except urllib.error.HTTPError as e:
            r = e
        with r:
            return r.status, json.loads(r.read())

    def draft(self):
        return {
            "id": str(uuid.uuid4()),
            "title": "Simpler signup",
            "outcome": "Ask for less data.",
            "scope": "Signup form only.",
            "context": "Preserve existing identity rules.",
        }

    def test_create_sdk_source_and_restart(self):
        draft = self.draft()
        status, created = self.call("/api/warrants", "POST", draft)
        self.assertEqual(status, 201, created)
        self.assertEqual(created["revision"], 1)
        self.assertEqual(created["work_state"], "draft")
        self.assertFalse(created["qualified"])
        self.assertIn("oh.war/document/1.0.0-rc.3", created["source"])
        self.assertIn("Ask for less data.", created["source"])
        self.stop()
        self.start()
        status, loaded = self.call("/api/warrants/" + draft["id"])
        self.assertEqual(status, 200)
        self.assertEqual(loaded, created)
        status, listing = self.call("/api/warrants")
        self.assertEqual(status, 200)
        self.assertEqual(listing["warrants"][0]["id"], draft["id"])

    def test_revision_conflict_preserves_history(self):
        draft = self.draft()
        _, first = self.call("/api/warrants", "POST", draft)
        edit = {
            **draft,
            "outcome": "Only ask for email.",
            "expected_source_sha256": first["source_sha256"],
        }
        status, second = self.call("/api/warrants/" + draft["id"], "PUT", edit)
        self.assertEqual(status, 200, second)
        self.assertEqual(second["revision"], 2)
        self.assertIn("Only ask for email.", second["source"])
        status, conflict = self.call("/api/warrants/" + draft["id"], "PUT", edit)
        self.assertEqual(status, 409, conflict)
        status, old = self.call("/api/warrants/" + draft["id"] + "/revisions/1")
        self.assertEqual(status, 200)
        self.assertEqual(old, first)
        self.stop()
        self.start()
        _, current = self.call("/api/warrants/" + draft["id"])
        self.assertEqual(current, second)

    def test_access_and_authority_fields_refuse(self):
        for headers, status in [
            ({"Authorization": ""}, 401),
            ({"Authorization": "Bearer wrong"}, 401),
            ({"Origin": "https://example.com"}, 403),
            ({"Host": "evil.invalid"}, 403),
        ]:
            with self.subTest(headers=headers):
                self.assertEqual(
                    self.call("/api/warrants", "POST", self.draft(), headers)[0], status
                )
        self.assertEqual(
            self.call("/api/warrants", "POST", {**self.draft(), "qualified": True})[0],
            400,
        )
        self.assertEqual(
            self.call("/api/warrants", "POST", {**self.draft(), "id": "../../escape"})[
                0
            ],
            400,
        )
        self.assertEqual(
            self.call("/api/warrants", "POST", {**self.draft(), "scope": "x" * 70000})[
                0
            ],
            413,
        )
        self.assertEqual(self.call("/api/warrants")[1]["warrants"], [])

    def test_board_is_authenticated_read_only_and_matches_cli(self):
        status, result = self.call("/api/board", headers={"Authorization": "Bearer wrong"})
        self.assertEqual(status, 401)
        status, result = self.call("/api/board")
        self.assertEqual(status, 200, result)
        expected = subprocess.run([WAR, "board", "--json"], cwd=REPO,
                                  capture_output=True, check=True)
        expected = json.loads(expected.stdout)["result"]
        self.assertEqual(result, expected)
        self.assertEqual(result["schema"], "oh.war/board-draft/v1")
        self.assertTrue(result["corpus"]["warrants"])
        status, _ = self.call("/api/board", method="POST", data={})
        self.assertEqual(status, 404)

    def test_project_inventory_keeps_legacy_and_work_separate(self):
        status, project = self.call("/api/project")
        self.assertEqual(status, 200, project)
        self.assertFalse(project["all_remaining_reconciled"])
        rows = {r["alias"]: r for r in project["warrants"]}
        report = json.loads(
            (
                REPO / "docs/warrants/OW-WAR-0088/implementation/progress.json"
            ).read_text()
        )
        self.assertEqual(rows["OW-WAR-0088"]["work_state"], report["work_state"])
        self.assertIn("legacy_phase", rows["OW-WAR-0088"])
        self.assertTrue(
            all(r["route"] and r["reconciled"] is False for r in rows.values())
        )
        self.assertEqual(project["total"], len(rows))
        self.assertTrue(all(not r["qualification_assessed"] for r in rows.values()))

    def test_concurrent_edits_have_one_winner(self):
        from concurrent.futures import ThreadPoolExecutor

        draft = self.draft()
        _, first = self.call("/api/warrants", "POST", draft)
        edit = {**draft, "expected_source_sha256": first["source_sha256"]}
        with ThreadPoolExecutor(max_workers=2) as pool:
            results = list(
                pool.map(
                    lambda _: self.call("/api/warrants/" + draft["id"], "PUT", edit),
                    range(2),
                )
            )
        self.assertEqual(sorted(r[0] for r in results), [200, 409])
        self.assertEqual(self.call("/api/warrants/" + draft["id"])[1]["revision"], 2)

    def test_corrupt_revision_and_second_writer_refuse(self):
        draft = self.draft()
        self.call("/api/warrants", "POST", draft)
        second = subprocess.run(
            check=False,
            args=[
                sys.executable,
                str(APP),
                "--war",
                WAR,
                "--repo",
                str(REPO),
                "--state",
                str(self.root / "state"),
                "--port",
                "0",
                "--session-file",
                str(self.root / "other.json"),
            ],
            capture_output=True,
            timeout=10,
        )
        self.assertNotEqual(second.returncode, 0)
        self.assertIn(b"already in use", second.stderr)
        self.assertFalse((self.root / "other.json").exists())
        path = self.root / "state" / (draft["id"] + ".00000001.json")
        record = json.loads(path.read_text())
        record["payload"] += "corrupt"
        path.write_text(json.dumps(record))
        status, result = self.call("/api/warrants/" + draft["id"])
        self.assertEqual(status, 409, result)
        self.assertEqual(self.call("/api/warrants", "POST", self.draft())[0], 409)

    def test_duplicate_json_and_symlink_revision_refuse(self):
        req = urllib.request.Request(
            self.url + "/api/warrants",
            data=b'{"id":"a","id":"b"}',
            headers={
                "Authorization": "Bearer " + self.token,
                "Content-Type": "application/json",
            },
        )
        with self.assertRaises(urllib.error.HTTPError) as context:
            urllib.request.urlopen(req, timeout=10)
        self.assertEqual(context.exception.code, 400)
        context.exception.close()
        draft = self.draft()
        self.call("/api/warrants", "POST", draft)
        path = self.root / "state" / (draft["id"] + ".00000001.json")
        original = path.read_bytes()
        outside = self.root / "original.json"
        outside.write_bytes(original)
        path.unlink()
        path.symlink_to(outside)
        self.assertEqual(self.call("/api/warrants/" + draft["id"])[0], 409)
        self.assertEqual(outside.read_bytes(), original)

    def test_field_only_corruption_refuses(self):
        draft = self.draft()
        self.call("/api/warrants", "POST", draft)
        path = self.root / "state" / (draft["id"] + ".00000001.json")
        stored = json.loads(path.read_text())
        # Alter authoring fields, preserving the source bytes and source digest.
        if "payload" in stored:
            record = json.loads(stored["payload"])
            record["title"] = "Corrupted title"
            stored["payload"] = json.dumps(record)
        else:
            stored["title"] = "Corrupted title"
        path.write_text(json.dumps(stored))
        self.assertEqual(self.call("/api/warrants/" + draft["id"])[0], 409)


if __name__ == "__main__":
    unittest.main()
