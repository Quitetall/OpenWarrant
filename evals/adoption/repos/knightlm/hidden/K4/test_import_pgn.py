"""K4: `import-pgn <file.pgn> [--game N]` emits a GameRecord."""
import json

from conftest import START_FEN

RUY_LOPEZ = """[Event "Casual game"]
[Site "https://lichess.org/abcd1234"]
[Date "2024.01.01"]
[Round "-"]
[White "A"]
[Black "B"]
[Result "1-0"]

1. e4 {the king's pawn} e5 2. Nf3 $1 Nc6 (2... d6 3. d4 (3. Bc4 Be7) exd4) 3. Bb5 a6 ; old line
4. Ba4 Nf6 5. O-O Be7 6. Re1!? b5 7. Bb3 d6 8. c3 O-O 1-0
"""

ENDGAME = """[Event "Study"]
[Site "?"]
[Result "*"]
[SetUp "1"]
[FEN "4k3/8/8/8/8/8/4P3/4K3 b - - 3 40"]

40... Kd7 41. e4 Ke6 42. Ke2 *
"""


def refused(result):
    return result.returncode != 0 and result.stdout.strip() == ""


def write(tmp_path, text, name="games.pgn"):
    path = tmp_path / name
    path.write_text(text)
    return path


def test_first_game_record_fields(ok, tmp_path):
    record = ok("import-pgn", write(tmp_path, RUY_LOPEZ + "\n" + ENDGAME))
    assert record == {
        "schema_version": 1,
        "game_id": "https://lichess.org/abcd1234",
        "source": "pgn:games.pgn#1",
        "initial_fen": START_FEN,
        "history_complete": True,
        "moves": ["e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6", "b5a4", "g8f6",
                  "e1g1", "f8e7", "f1e1", "b7b5", "a4b3", "d7d6", "c2c3", "e8g8"],
    }


def test_output_replays_with_compile(ok, tmp_path):
    out = tmp_path / "record.json"
    out.write_text(json.dumps(ok("import-pgn", write(tmp_path, RUY_LOPEZ))))
    packet = ok("compile", out)
    assert packet["fen"] == "r1bq1rk1/2p1bppp/p1np1n2/1p2p3/4P3/1BP2N2/PP1P1PPP/RNBQR1K1 w - - 1 9"
    assert packet["history_complete"] is True


def test_setup_game_uses_fen_and_never_claims_history(ok, tmp_path):
    record = ok("import-pgn", write(tmp_path, RUY_LOPEZ + "\n" + ENDGAME), "--game", 2)
    assert record["initial_fen"] == "4k3/8/8/8/8/8/4P3/4K3 b - - 3 40"
    assert record["history_complete"] is False
    assert record["moves"] == ["e8d7", "e2e4", "d7e6", "e1e2"]
    assert record["game_id"] == "games.pgn#2"
    assert record["source"] == "pgn:games.pgn#2"


def test_fen_tag_of_standard_start_still_incomplete(ok, tmp_path):
    text = f'[FEN "{START_FEN}"]\n[SetUp "1"]\n\n1. d4 d5 *\n'
    record = ok("import-pgn", write(tmp_path, text, "setup.pgn"))
    assert record["initial_fen"] == START_FEN
    assert record["history_complete"] is False
    assert record["moves"] == ["d2d4", "d7d5"]


def test_missing_tags_fall_back_to_file_reference(ok, tmp_path):
    record = ok("import-pgn", write(tmp_path, "1. e4 e5 *\n", "bare.pgn"))
    assert record["game_id"] == "bare.pgn#1"
    assert record["source"] == "pgn:bare.pgn#1"
    assert record["history_complete"] is True


def test_movetext_variants_are_accepted(ok, tmp_path):
    text = ("1.e4 e5 2.Ngf3 Nc6 3.Bc4 Bc5 4.0-0 d6!? 5.c3 Bg4?! 6.d4 exd4 7.cxd4 Bb4"
            " 8.Nc3 Nf6 9.Qb3 Qd7 10.Bb5 0-0-0 11.Bxc6 Qxc6 12.d5 Kb8 *\n")
    record = ok("import-pgn", write(tmp_path, text))
    assert record["moves"][:4] == ["e2e4", "e7e5", "g1f3", "b8c6"]
    assert record["moves"][6] == "e1g1"
    assert record["moves"][19] == "e8c8"
    assert record["moves"][-2:] == ["d4d5", "c8b8"]
    assert len(record["moves"]) == 24


def test_illegal_move_names_move_number_and_side(cli, tmp_path):
    result = cli("import-pgn", write(tmp_path, "1. e4 e5 2. Ke3 Nc6 *\n"))
    assert refused(result)
    assert 'move 2 (white): illegal or ambiguous SAN "Ke3"' in result.stderr
    black = cli("import-pgn", write(tmp_path, "1. e4 Ke7 *\n", "black.pgn"))
    assert refused(black)
    assert 'move 1 (black): illegal or ambiguous SAN "Ke7"' in black.stderr


def test_ambiguous_move_is_refused(cli, tmp_path):
    result = cli("import-pgn", write(tmp_path, "1. d4 d5 2. Nf3 Nf6 3. Nd2 *\n"))
    assert refused(result)
    assert 'move 3 (white): illegal or ambiguous SAN "Nd2"' in result.stderr


def test_setup_move_numbers_come_from_the_fen(cli, tmp_path):
    text = '[FEN "4k3/8/8/8/8/8/4P3/4K3 b - - 3 40"]\n\n40... Kd7 41. e5 *\n'
    result = cli("import-pgn", write(tmp_path, text))
    assert refused(result)
    assert 'move 41 (white): illegal or ambiguous SAN "e5"' in result.stderr


def test_result_tag_must_match_termination(cli, tmp_path):
    result = cli("import-pgn", write(tmp_path, '[Result "1-0"]\n\n1. e4 e5 0-1\n'))
    assert refused(result)
    assert "disagrees with termination marker" in result.stderr


def test_game_index_out_of_range(cli, tmp_path):
    path = write(tmp_path, RUY_LOPEZ + "\n" + ENDGAME)
    for index in (3, 0):
        result = cli("import-pgn", path, "--game", index)
        assert refused(result)
        assert "out of range" in result.stderr


def test_missing_termination_marker_is_refused(cli, tmp_path):
    result = cli("import-pgn", write(tmp_path, "1. e4 e5 2. Nf3\n"))
    assert refused(result)
    assert "missing game termination marker" in result.stderr
