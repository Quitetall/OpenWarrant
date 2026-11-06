"""K2: same-coloured bishops dead-position recognizer in `compile` facts."""
import json

import pytest

# Square colours: a1 is dark. c3, b2, e7, d6 are dark; f1, c8, h3, f7, c2 are light.
DEAD = {
    "one dark bishop each": "8/4b3/4k3/8/8/2B5/8/4K3 w - - 0 1",
    "two dark bishops vs bare king, black to move": "8/8/4k3/8/8/2B5/1B6/4K3 b - - 0 1",
    "light bishops for both sides": "2b1k3/8/8/8/8/7b/8/4KB2 w - - 0 1",
    "three dark bishops one side, one dark the other": "8/8/3bk3/8/8/2B5/1B6/B3K3 w - - 0 1",
}
UNKNOWN = {
    "opposite-coloured bishops": "8/5b2/4k3/8/8/2B5/8/4K3 w - - 0 1",
    "bishop pair on both colours vs bare king": "8/8/4k3/8/8/2B5/2B5/4K3 w - - 0 1",
    "two knights vs bare king": "8/8/4k3/8/8/2N5/2N5/4K3 w - - 0 1",
    "bishop vs knight": "8/5n2/4k3/8/8/2B5/8/4K3 w - - 0 1",
    "bishop and knight vs bare king": "8/8/4k3/8/8/2B5/2N5/4K3 w - - 0 1",
    "same-coloured bishops plus a pawn": "8/4b3/4k3/8/8/2B5/7P/4K3 w - - 0 1",
}
ALREADY_DEAD = {
    "bare kings": "8/8/4k3/8/8/8/8/4K3 w - - 0 1",
    "knight vs bare king": "8/8/4k3/8/8/2N5/8/4K3 w - - 0 1",
    "light bishop vs bare king": "8/8/4k3/8/8/8/8/4KB2 b - - 0 1",
}


@pytest.mark.parametrize("fen", DEAD.values(), ids=DEAD.keys())
def test_same_coloured_bishops_are_dead(ok, game, fen):
    packet = ok("compile", game(fen), "--legal-choices")
    assert packet["facts"]["dead_position"] is True
    assert packet["facts"]["terminal_reason"] == "proven_dead_position"
    assert packet["legal_moves"] == []


@pytest.mark.parametrize("fen", UNKNOWN.values(), ids=UNKNOWN.keys())
def test_other_material_stays_unknown(ok, game, fen):
    packet = ok("compile", game(fen), "--legal-choices")
    assert packet["facts"]["dead_position"] is None  # null: unknown, never false
    assert packet["facts"]["terminal_reason"] is None
    assert packet["legal_moves"]


@pytest.mark.parametrize("fen", ALREADY_DEAD.values(), ids=ALREADY_DEAD.keys())
def test_elementary_cases_still_dead(ok, game, fen):
    facts = ok("compile", game(fen))["facts"]
    assert facts["dead_position"] is True
    assert facts["terminal_reason"] == "proven_dead_position"


def test_dead_position_reached_by_a_capture(ok, game, cli):
    # White Bc3xb4 removes the knight, leaving dark-squared bishops b4 and d6 only.
    path = game("8/8/3bk3/8/1n6/2B5/8/4K3 w - - 0 1", moves=["c3b4"])
    packet = ok("compile", path)
    assert packet["facts"]["dead_position"] is True
    assert packet["facts"]["terminal_reason"] == "proven_dead_position"
    simulated = cli("simulate", path, "e1d1")
    assert simulated.returncode == 0
    assert json.loads(simulated.stdout)[0]["legal"] is False


def test_start_position_is_unknown_not_false(ok, game):
    assert ok("compile", game())["facts"]["dead_position"] is None
