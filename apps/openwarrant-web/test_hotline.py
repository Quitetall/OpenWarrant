# SPDX-License-Identifier: Apache-2.0
"""Protocol boundaries; HTTP, scheduling and harness integration tested separately."""
import hashlib
import unittest
from hotline import HotlineError, answer, digest, question, responders


class HotlineContracts(unittest.TestCase):
    def setUp(self):
        self.warrant = "00000000-0000-0000-0000-000000000001"
        self.attempt = {"attempt_id": "attempt", "warrant_id": self.warrant,
                        "source_sha256": "a" * 64, "policy": {"verified_start": False}}
        self.result = {"schema": "oh.war/execution-question/v1", "attempt_id": "attempt",
                       "source_sha256": "a" * 64, "notes": "Checkpoint committed",
                       "next_steps": ["Answer before continuing"], "question": {
                           "kind": "technical", "text": "Which existing parser handles this?",
                           "direct_human": False, "affected_stages": ["STAGE-001"]}}
        self.token = "fixture-responder-credential-00000001"
        self.row = {"id": "configured-adviser", "kind": "ai", "governing_warrants": [],
                    "token_sha256": hashlib.sha256(self.token.encode()).hexdigest()}

    def rows(self):
        return responders({"schema": "oh.war/hotline-config/v1", "responders": [self.row]})

    def test_exact_question_basis_and_credential_derived_answer(self):
        q = question(self.result, self.attempt, "b" * 40)
        self.result["question"]["text"] = "Changed caller input"
        self.assertNotEqual(q["question"]["text"], self.result["question"]["text"])
        fields = {"question_sha256": digest(q), "answer": "Use the existing SDK parser.", "evidence": ["src/parser.rs"]}
        got = answer(fields, self.token, self.rows(), q)
        self.assertEqual(got["respondent"], "configured-adviser")
        self.assertFalse(got["qualified"])
        for altered in ({**fields, "actor": "owner"}, {**fields, "question_sha256": "0" * 64}):
            with self.assertRaises(HotlineError): answer(altered, self.token, self.rows(), q)
        with self.assertRaises(HotlineError): answer(fields, "wrong" * 10, self.rows(), q)

    def test_governing_scope_and_direct_human_are_distinct(self):
        self.result["question"]["kind"] = "governing"
        q = question(self.result, self.attempt, "b" * 40)
        fields = {"question_sha256": digest(q), "answer": "Approved decision within delegated scope", "evidence": []}
        with self.assertRaises(HotlineError): answer(fields, self.token, self.rows(), q)
        self.row["governing_warrants"] = [self.warrant]
        self.assertEqual(answer(fields, self.token, self.rows(), q)["respondent_kind"], "ai")
        self.result["question"]["direct_human"] = True
        q = question(self.result, self.attempt, "b" * 40); fields["question_sha256"] = digest(q)
        with self.assertRaises(HotlineError): answer(fields, self.token, self.rows(), q)
        self.row["kind"] = "human"
        self.assertEqual(answer(fields, self.token, self.rows(), q)["respondent_kind"], "human")

    def test_subject_checkpoint_and_claim_refusals(self):
        with self.assertRaises(HotlineError): question(self.result, self.attempt, "HEAD")
        with self.assertRaises(HotlineError): question({**self.result, "qualified": True}, self.attempt, "b" * 40)
        with self.assertRaises(HotlineError): question({**self.result, "source_sha256": "c" * 64}, self.attempt, "b" * 40)
        q = question(self.result, self.attempt, "b" * 40)
        changed = question(self.result, {**self.attempt, "policy": {"verified_start": True}}, "b" * 40)
        self.assertNotEqual(digest(q), digest(changed))
        self.assertNotEqual(digest(q), digest(question(self.result, self.attempt, "c" * 40)))

    def test_duplicate_credentials_and_wildcard_authority_refuse(self):
        with self.assertRaises(HotlineError): responders({"schema": "oh.war/hotline-config/v1", "responders": [self.row, {**self.row, "id": "owner"}]})
        self.row["governing_warrants"] = ["*"]
        with self.assertRaises(HotlineError): self.rows()
        self.assertEqual(responders({"schema": "oh.war/hotline-config/v1", "responders": []}), [])


if __name__ == "__main__":
    unittest.main()
