"""K7: `encode <game.json> [--relative]` flattens the current position."""
from conftest import START_FEN, refused

KEYS = {"fen", "side_to_move", "relative", "squares", "castling", "en_passant",
        "halfmove_clock", "fullmove_number", "repetition_count"}
CASTLING_KEYS = {"white_king_side", "white_queen_side", "black_king_side", "black_queen_side"}


def board(*ranks):
    """Expected `squares` from eight FEN-like rank strings, rank 8 first ('.' empty)."""
    assert len(ranks) == 8 and all(len(r) == 8 for r in ranks)
    return [c for rank in ranks for c in rank]


def index(square):
    """Index of a square in `squares`: a8 is 0, h8 is 7, a1 is 56, h1 is 63."""
    file, rank = ord(square[0]) - ord("a"), int(square[1])
    return (8 - rank) * 8 + file


def test_start_position_exact(ok, game):
    out = ok("encode", game())
    assert set(out) == KEYS
    assert out == {
        "fen": START_FEN,
        "side_to_move": "white",
        "relative": False,
        "squares": board("rnbqkbnr", "pppppppp", "........", "........",
                         "........", "........", "PPPPPPPP", "RNBQKBNR"),
        "castling": {k: True for k in CASTLING_KEYS},
        "en_passant": None,
        "halfmove_clock": 0,
        "fullmove_number": 1,
        "repetition_count": 1,
    }


def test_square_order_after_moves(ok, game):
    out = ok("encode", game(moves=["e2e4", "g8f6", "g1f3"]))
    squares = out["squares"]
    assert len(squares) == 64 and all(isinstance(s, str) and len(s) == 1 for s in squares)
    assert squares[index("e4")] == "P" and squares[index("e2")] == "."
    assert squares[index("f6")] == "n" and squares[index("g8")] == "."
    assert squares[index("f3")] == "N" and squares[index("g1")] == "."
    assert squares[0] == "r" and squares[7] == "r" and squares[56] == "R" and squares[63] == "R"
    assert squares[4] == "k" and squares[60] == "K"


def test_fen_and_counters_match_compile(ok, game):
    path = game(moves=["g1f3", "g8f6", "f3g1", "f6g8", "g1f3"])
    out = ok("encode", path)
    packet = ok("compile", path)
    assert out["fen"] == packet["fen"]
    assert out["side_to_move"] == "black"
    assert out["halfmove_clock"] == 5
    assert out["fullmove_number"] == 3
    assert out["repetition_count"] == packet["facts"]["repetition_count"] == 2


def test_repetition_count_null_without_complete_history(ok, game):
    out = ok("encode", game(complete=False))
    assert out["repetition_count"] is None


def test_castling_rights_follow_the_ledger(ok, game):
    out = ok("encode", game(moves=["e2e4", "e7e5", "e1e2", "a7a6"]))
    assert out["castling"] == {"white_king_side": False, "white_queen_side": False,
                               "black_king_side": True, "black_queen_side": True}
    out = ok("encode", game(fen="r3k2r/8/8/8/8/8/8/R3K2R w Kq - 0 1"))
    assert out["castling"] == {"white_king_side": True, "white_queen_side": False,
                               "black_king_side": False, "black_queen_side": True}


def test_en_passant_only_when_capture_is_legal(ok, game):
    usable = ok("encode", game(moves=["e2e4", "a7a6", "e4e5", "d7d5"]))
    assert usable["en_passant"] == "d6"
    no_capturer = ok("encode", game(moves=["e2e4"]))
    assert no_capturer["en_passant"] is None
    # The b5 pawn could only capture on c6 by exposing its king on the fifth rank.
    pinned = ok("encode", game(fen="4k3/2p5/8/KP5r/8/8/8/8 b - - 0 1", moves=["c7c5"]))
    assert pinned["en_passant"] is None


def test_relative_is_absolute_when_white_to_move(ok, game):
    path = game(moves=["e2e4", "e7e5"])
    absolute = ok("encode", path)
    relative = ok("encode", path, "--relative")
    assert relative["relative"] is True and absolute["relative"] is False
    assert {**relative, "relative": False} == absolute


def test_relative_mirrors_and_swaps_colours_for_black(ok, game):
    path = game(moves=["e2e4"])
    out = ok("encode", path, "--relative")
    assert out["fen"] == ok("compile", path)["fen"]
    assert out["fen"].startswith("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b ")
    assert out["side_to_move"] == "black"
    assert out["squares"] == board("rnbqkbnr", "pppp.ppp", "........", "....p...",
                                   "........", "........", "PPPPPPPP", "RNBQKBNR")


def test_relative_mirrors_castling_and_en_passant(ok, game):
    rights = ok("encode", game(fen="r3k2r/8/8/8/8/8/8/R3K2R b Kq - 0 1"), "--relative")
    assert rights["castling"] == {"white_king_side": False, "white_queen_side": True,
                                  "black_king_side": True, "black_queen_side": False}
    assert rights["squares"][index("e1")] == "K" and rights["squares"][index("e8")] == "k"
    ep = ok("encode", game(fen="4k3/8/8/8/4p3/8/3P4/4K3 w - - 0 1", moves=["d2d4"]))
    assert ep["en_passant"] == "d3"
    mirrored = ok("encode", game(fen="4k3/8/8/8/4p3/8/3P4/4K3 w - - 0 1", moves=["d2d4"]),
                  "--relative")
    assert mirrored["en_passant"] == "d6"
    assert mirrored["squares"][index("e5")] == "P" and mirrored["squares"][index("d5")] == "p"
    assert mirrored["squares"][index("e1")] == "K" and mirrored["squares"][index("e8")] == "k"


def test_invalid_records_are_refused(cli, game, tmp_path):
    assert refused(cli("encode", game(moves=["e2e5"])))
    bad = tmp_path / "unknown-field.json"
    bad.write_text('{"schema_version": 1, "game_id": "g", "source": "s", "initial_fen": "'
                   + START_FEN + '", "history_complete": true, "moves": [], "result": "*"}')
    assert refused(cli("encode", bad))
    assert refused(cli("encode", tmp_path / "missing.json"))
