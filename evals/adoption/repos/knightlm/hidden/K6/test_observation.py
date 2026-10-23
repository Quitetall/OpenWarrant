"""K6: `verify-observation` lists per-square mismatches and refuses malformed placements."""
import json

import pytest

START_PLACEMENT = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR"


def refused(result):
    return result.returncode != 0 and result.stdout.strip() == ""


@pytest.fixture
def observation(tmp_path):
    counter = iter(range(10**6))

    def write(placement, **overrides):
        path = tmp_path / f"observation-{next(counter)}.json"
        body = {"schema_version": 1, "image_id": "board.svg", "placement": placement}
        body.update(overrides)
        path.write_text(json.dumps(body))
        return path
    return write


def test_match_reports_empty_mismatch_list(ok, game, observation):
    assert ok("verify-observation", game(), observation(START_PLACEMENT)) == {
        "placement_matches": True, "repaired": False, "mismatches": []}


def test_match_after_replayed_moves(ok, game, observation):
    report = ok("verify-observation", game(moves=["e2e4", "e7e5"]),
                observation("rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR"))
    assert report["placement_matches"] is True and report["mismatches"] == []


def test_mismatch_is_reported_then_fails(cli, game, observation):
    result = cli("verify-observation", game(),
                 observation("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR"))
    assert result.returncode != 0
    assert "perception failure" in result.stderr
    assert json.loads(result.stdout) == {
        "placement_matches": False,
        "repaired": False,
        "mismatches": [
            {"square": "e2", "expected": "P", "observed": None},
            {"square": "e4", "expected": None, "observed": "P"},
        ],
    }


def test_mismatches_are_in_a1_to_h8_order(cli, game, observation):
    result = cli("verify-observation", game(),
                 observation("1nbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBKKBN1"))
    assert result.returncode != 0
    assert json.loads(result.stdout)["mismatches"] == [
        {"square": "d1", "expected": "Q", "observed": "K"},
        {"square": "h1", "expected": "R", "observed": None},
        {"square": "a8", "expected": "r", "observed": None},
    ]


def test_colour_confusion_is_a_mismatch(cli, game, observation):
    result = cli("verify-observation", game(),
                 observation("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNr"))
    assert result.returncode != 0
    assert json.loads(result.stdout)["mismatches"] == [
        {"square": "h1", "expected": "R", "observed": "r"}]


def test_empty_board_lists_every_piece(cli, game, observation):
    result = cli("verify-observation", game(), observation("8/8/8/8/8/8/8/8"))
    assert result.returncode != 0
    report = json.loads(result.stdout)
    assert report["placement_matches"] is False and report["repaired"] is False
    assert len(report["mismatches"]) == 32
    assert all(m["observed"] is None for m in report["mismatches"])
    assert report["mismatches"][0] == {"square": "a1", "expected": "R", "observed": None}
    assert report["mismatches"][-1] == {"square": "h8", "expected": "r", "observed": None}


@pytest.mark.parametrize("placement", [
    "8/8/8/8/8/8/8",
    "8/8/8/8/8/8/8/8/8",
    "9/8/8/8/8/8/8/8",
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNX",
    "rnbqkbnr/ppppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR",
    "rnbqkbnr/ppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR",
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "",
])
def test_malformed_placement_is_refused(cli, game, observation, placement):
    result = cli("verify-observation", game(), observation(placement))
    assert refused(result)
    assert "malformed observation placement" in result.stderr


def test_other_observation_errors_still_refused(cli, game, observation):
    assert refused(cli("verify-observation", game(), observation(START_PLACEMENT, schema_version=2)))
    assert refused(cli("verify-observation", game(), observation(START_PLACEMENT, image_id="")))
