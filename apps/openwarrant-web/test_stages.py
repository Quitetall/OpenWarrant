# SPDX-License-Identifier: Apache-2.0
import unittest
from stages import StageError, frontier, validate, plan


class StagePlanningTests(unittest.TestCase):
    graph = {"api": [], "ui": ["api"], "integration": ["ui"], "docs": []}

    def states(self, **kwargs):
        return {r["stage"]: r["state"] for r in frontier(self.graph, **kwargs)}

    def test_block_propagates_but_independent_stage_remains_ready(self):
        self.assertEqual(self.states(question_blocks=["api"]), {
            "api": "blocked", "ui": "blocked", "integration": "blocked", "docs": "ready"})
        self.assertEqual(self.states(completed=["api"], question_blocks=["ui"]), {
            "api": "completed", "ui": "blocked", "integration": "blocked", "docs": "ready"})
        self.assertTrue(all(not r["dispatch_permitted"] for r in frontier(self.graph)))

    def test_one_writer_serializes_independent_stages_and_new_block_requests_stop(self):
        self.assertEqual(self.states(writer="api")["docs"], "waiting-for-writer")
        self.assertEqual(self.states(writer="api")["ui"], "waiting-for-prerequisite")
        states = self.states(writer="api", question_blocks=["api"])
        self.assertEqual(states["api"], "stop-required")
        self.assertEqual(states["docs"], "waiting-for-writer")
        self.assertEqual(self.states(completed=["api"])["ui"], "ready")

    def test_bad_graphs_and_contradictory_observations_refuse(self):
        for graph in ({}, {"a": ["missing"]}, {"a": ["a"]}, {"a": ["b"], "b": ["a"]},
                      {"a": [], "b": ["a", "a"]}, {str(n): [] for n in range(65)}):
            with self.subTest(graph=graph), self.assertRaises(StageError): validate(graph)
        for facts in ({"completed": ["missing"]}, {"question_blocks": ["missing"]},
                      {"completed": "api"}, {"writer": "ui"},
                      {"completed": ["ui"]}, {"completed": ["api"], "writer": "api"},
                      {"completed": ["api"], "question_blocks": ["api"]}):
            with self.subTest(facts=facts), self.assertRaises(StageError): frontier(self.graph, **facts)

    def test_stage_policy_requires_outcome_checks_and_valid_graph(self):
        import copy
        config = {"schema": "oh.war/execution-stage-plan/v1", "stages": {
            "api": {"title": "API", "outcome": "Required response and refusal behavior",
                    "dependencies": [], "checks": [["python", "test_api.py"]]},
            "ui": {"title": "UI", "outcome": "Show the API result", "dependencies": ["api"],
                   "checks": [["python", "test_ui.py"]]}}}
        parsed = plan(config)
        config["stages"]["api"]["title"] = "Caller mutated"
        self.assertEqual(parsed["stages"]["api"]["title"], "API")
        for key, value in (("checks", []), ("checks", [["bad\0command"]]),
                           ("dependencies", ["missing"]), ("outcome", "")):
            bad = copy.deepcopy(parsed); bad["stages"]["api"][key] = value
            with self.subTest(key=key), self.assertRaises(StageError): plan(bad)
        with self.assertRaises(StageError): plan({**parsed, "qualified": True})
        with self.assertRaises(StageError): plan({**parsed, "schema": "future-schema"})

    def test_plan_is_deterministic_and_does_not_modify_input(self):
        graph = {"b": ["a"], "a": []}
        self.assertEqual(frontier(graph), frontier({"a": [], "b": ["a"]}))
        self.assertEqual(graph, {"b": ["a"], "a": []})


if __name__ == "__main__":
    unittest.main()
