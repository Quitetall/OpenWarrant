"""K5: `export-pgn <game.json>` writes PGN that `import-pgn` reads back."""
import json

import pytest

from conftest import START_FEN

ROSTER = ('[Event "?"]\n[Site "{site}"]\n[Date "????.??.??"]\n[Round "?"]\n'
          '[White "?"]\n[Black "?"]\n[Result "{result}"]\n')
CYCLE = ["g1f3", "g8f6", "f3g1", "f6g8"]


def refused(result):
    return result.returncode != 0 and result.stdout.strip() == ""


def export(cli, path):
    result = cli("export-pgn", path)
    assert result.returncode == 0, result.stderr
    return result.stdout


def movetext(pgn):
    head, _, body = pgn.partition("\n\n")
    return body.split()


def test_exact_output_for_fools_mate(cli, game):
    path = game(moves=["f2f3", "e7e5", "g2g4", "d8h4"], game_id="fool")
    assert export(cli, path) == ROSTER.format(site="fool", result="0-1") + "\n1. f3 e5 2. g4 Qh4# 0-1\n"


def test_empty_game(cli, game):
    assert export(cli, game(game_id="empty")) == ROSTER.format(site="empty", result="*") + "\n*\n"


def test_setup_position_with_black_to_move(cli, game):
    fen = "4k3/8/8/8/8/8/4P3/4K3 b - - 3 40"
    pgn = export(cli, game(fen, moves=["e8d7", "e2e4"], complete=False, game_id="study"))
    assert pgn == (ROSTER.format(site="study", result="*")
                   + f'[SetUp "1"]\n[FEN "{fen}"]\n\n40... Kd7 41. e4 *\n')


def test_incomplete_history_from_start_still_writes_fen(cli, game):
    pgn = export(cli, game(complete=False, moves=["e2e4"]))
    assert f'[SetUp "1"]\n[FEN "{START_FEN}"]\n' in pgn
    complete = export(cli, game(moves=["e2e4"]))
    assert "SetUp" not in complete and "FEN" not in complete


@pytest.mark.parametrize("fen,moves,result", [
    (START_FEN, ["e2e4", "e7e5", "d1h5", "b8c6", "f1c4", "g8f6", "h5f7"], "1-0"),
    (START_FEN, ["f2f3", "e7e5", "g2g4", "d8h4"], "0-1"),
    ("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", [], "1/2-1/2"),
    ("8/8/4k3/8/8/8/3q4/3K4 w - - 0 1", ["d1d2"], "1/2-1/2"),
    (START_FEN, CYCLE * 4, "1/2-1/2"),
    (START_FEN, CYCLE * 2, "*"),
    (START_FEN, ["e2e4"], "*"),
], ids=["white-mates", "black-mates", "stalemate", "dead-after-capture",
        "fivefold", "threefold-only-claimable", "ongoing"])
def test_result_is_computed_from_final_position(cli, game, fen, moves, result):
    pgn = export(cli, game(fen, moves=moves, complete=True))
    assert f'[Result "{result}"]' in pgn
    assert movetext(pgn)[-1] == result


def test_scholars_mate_movetext(cli, game):
    pgn = export(cli, game(moves=["e2e4", "e7e5", "d1h5", "b8c6", "f1c4", "g8f6", "h5f7"]))
    assert pgn.endswith("\n\n1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7# 1-0\n")


def test_long_movetext_wraps_at_79_columns(cli, game):
    pgn = export(cli, game(moves=CYCLE * 4))
    lines = pgn.split("\n\n", 1)[1].rstrip("\n").split("\n")
    assert len(lines) >= 2
    assert all(len(line) <= 79 for line in lines)
    assert all(line == line.strip() and "  " not in line for line in lines)
    assert movetext(pgn)[:6] == ["1.", "Nf3", "Nf6", "2.", "Ng1", "Ng8"]
    assert len(movetext(pgn)) == 8 * 3 + 1
    for i, line in enumerate(lines[:-1]):
        assert len(line) + 1 + len(lines[i + 1].split()[0]) > 79  # greedy filling


@pytest.mark.parametrize("fen,moves,complete", [
    (START_FEN, ["e2e4", "d7d5", "e4d5", "c7c5", "d5c6", "g8f6", "c6b7", "c8d7", "b7a8q", "e7e6",
                 "g1f3", "f8e7", "f1e2", "e8g8", "e1g1"], True),
    ("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1", ["e1c1", "e8g8", "c1b1"], False),
    ("4k3/8/8/8/8/8/4P3/4K3 b - - 3 40", ["e8d7", "e2e4", "d7e6"], False),
    (START_FEN, ["b1c3", "g8f6", "g1f3", "b8c6", "c3e4", "f6e4", "f3e5", "c6e5"], False),
])
def test_round_trip_through_import(cli, ok, game, tmp_path, fen, moves, complete):
    record_path = game(fen, moves=moves, complete=complete, game_id="round-trip-7")
    pgn_path = tmp_path / "out.pgn"
    pgn_path.write_text(export(cli, record_path))
    back = ok("import-pgn", pgn_path)
    original = json.loads(record_path.read_text())
    for field in ("game_id", "initial_fen", "history_complete", "moves"):
        assert back[field] == original[field], field


def test_complete_setup_history_reimports_as_incomplete(cli, ok, game, tmp_path):
    fen = "4k3/8/8/8/8/8/4P3/4K3 b - - 3 40"
    pgn = export(cli, game(fen, moves=["e8d7"], complete=True))
    assert f'[FEN "{fen}"]' in pgn
    path = tmp_path / "setup.pgn"
    path.write_text(pgn)
    back = ok("import-pgn", path)
    assert back["initial_fen"] == fen and back["history_complete"] is False


def test_site_tag_escapes_quotes_and_backslashes(cli, ok, game, tmp_path):
    game_id = 'odd "id" \\ here'
    pgn = export(cli, game(game_id=game_id))
    assert '[Site "odd \\"id\\" \\\\ here"]' in pgn
    path = tmp_path / "odd.pgn"
    path.write_text(pgn)
    assert ok("import-pgn", path)["game_id"] == game_id


def test_invalid_records_are_refused(cli, game, tmp_path):
    assert refused(cli("export-pgn", game(moves=["e2e4", "e2e4"])))
    extra = tmp_path / "extra.json"
    extra.write_text(json.dumps({"schema_version": 1, "game_id": "g", "source": "s",
                                 "initial_fen": START_FEN, "history_complete": True,
                                 "moves": [], "result": "1-0"}))
    assert refused(cli("export-pgn", extra))
