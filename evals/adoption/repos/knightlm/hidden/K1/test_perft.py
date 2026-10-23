"""K1: `perft <game.json> <depth> [--divide]`."""
import pytest

from conftest import START_FEN

KIWIPETE = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"
POSITION_3 = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"
POSITION_5 = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8"


def refused(result):
    return result.returncode != 0 and result.stdout.strip() == ""


@pytest.mark.parametrize("fen,counts", [
    (START_FEN, [20, 400, 8902, 197281]),
    (KIWIPETE, [48, 2039, 97862]),
    (POSITION_3, [14, 191, 2812, 43238]),
    (POSITION_5, [44, 1486, 62379]),
])
def test_published_perft_counts(ok, game, fen, counts):
    path = game(fen)
    for depth, expected in enumerate(counts, start=1):
        report = ok("perft", path, depth)
        assert report["nodes"] == expected, (fen, depth)
        assert report["depth"] == depth
        assert report["fen"] == fen
        assert "divide" not in report


def test_depth_zero_counts_the_position_itself(ok, game):
    report = ok("perft", game(), 0)
    assert report == {"fen": START_FEN, "depth": 0, "nodes": 1}


def test_divide_lists_every_root_move_in_uci(ok, game):
    path = game()
    report = ok("perft", path, 2, "--divide")
    legal = ok("compile", path, "--legal-choices")["legal_moves"]
    assert sorted(report["divide"]) == sorted(legal)
    assert all(n == 20 for n in report["divide"].values())
    assert sum(report["divide"].values()) == report["nodes"] == 400


def test_divide_writes_castling_as_king_two_squares(ok, game):
    report = ok("perft", game(KIWIPETE), 1, "--divide")
    assert report["divide"]["e1g1"] == 1 and report["divide"]["e1c1"] == 1
    assert len(report["divide"]) == 48
    deeper = ok("perft", game(KIWIPETE), 2, "--divide")
    assert sum(deeper["divide"].values()) == 2039


def test_perft_starts_from_the_replayed_position(ok, game):
    path = game(moves=["e2e4", "c7c5"])
    report = ok("perft", path, 1)
    assert report["fen"] == ok("compile", path)["fen"]
    assert report["nodes"] == 30


def test_draw_adjudication_is_ignored(ok, game):
    dead = game("8/8/4k3/8/8/2B5/8/4K3 w - - 0 1")
    assert ok("compile", dead)["facts"]["terminal_reason"] == "proven_dead_position"
    assert ok("perft", dead, 1)["nodes"] == 15
    cycle = ["g1f3", "g8f6", "f3g1", "f6g8"]
    fivefold = game(moves=cycle * 4)
    assert ok("compile", fivefold)["facts"]["terminal_reason"] == "fivefold_repetition"
    assert ok("perft", fivefold, 2)["nodes"] == 400


def test_checkmate_has_no_successors(ok, game):
    mated = game(moves=["f2f3", "e7e5", "g2g4", "d8h4"])
    assert ok("perft", mated, 1)["nodes"] == 0
    assert ok("perft", mated, 3)["nodes"] == 0


def test_depth_above_six_is_refused(cli, game):
    assert refused(cli("perft", game(), 7))


def test_divide_at_depth_zero_is_refused(cli, game):
    assert refused(cli("perft", game(), 0, "--divide"))


def test_invalid_game_is_refused(cli, game):
    assert refused(cli("perft", game(moves=["e2e5"]), 1))
    assert refused(cli("perft", game(), "two"))
