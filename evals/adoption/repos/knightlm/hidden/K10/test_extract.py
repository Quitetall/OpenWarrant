"""K10: `extract <game.json> --from-ply N [--to-ply M] [--pgn]` cuts a ply range out of a game."""
import json

from conftest import START_FEN, refused

RUY = ["e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6"]


def test_middle_range_exact(ok, game):
    out = ok("extract", game(moves=RUY, game_id="ruy", source="lichess dump"),
             "--from-ply", 2, "--to-ply", 4)
    assert out == {
        "schema_version": 1,
        "game_id": "ruy@2-4",
        "source": "extract:lichess dump",
        "initial_fen": "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2",
        "history_complete": False,
        "moves": ["g1f3", "b8c6"],
    }


def test_to_ply_defaults_to_the_last_move(ok, game):
    out = ok("extract", game(moves=RUY, game_id="ruy"), "--from-ply", 4)
    assert out["game_id"] == "ruy@4-6"
    assert out["moves"] == ["f1b5", "a7a6"]


def test_from_ply_zero_still_never_claims_history(ok, game):
    out = ok("extract", game(moves=RUY, game_id="g"), "--from-ply", 0)
    assert out["initial_fen"] == START_FEN
    assert out["moves"] == RUY
    assert out["game_id"] == "g@0-6"
    assert out["history_complete"] is False


def test_empty_range_and_incomplete_source(ok, game):
    out = ok("extract", game(moves=RUY, game_id="g"), "--from-ply", 3, "--to-ply", 3)
    assert out["moves"] == [] and out["game_id"] == "g@3-3"
    assert out["history_complete"] is False
    out = ok("extract", game(moves=RUY, complete=False), "--from-ply", 1, "--to-ply", 2)
    assert out["history_complete"] is False and out["moves"] == ["e7e5"]


def test_initial_fen_and_final_position_match_compile(ok, game, tmp_path):
    for from_ply in range(len(RUY) + 1):
        out = ok("extract", game(moves=RUY), "--from-ply", from_ply)
        assert out["initial_fen"] == ok("compile", game(moves=RUY[:from_ply]))["fen"]
        path = tmp_path / f"extract-{from_ply}.json"
        path.write_text(json.dumps(out))
        packet = ok("compile", path)
        assert packet["fen"] == ok("compile", game(moves=RUY))["fen"]
        assert packet["history_complete"] is False
        assert packet["facts"]["repetition_count"] is None


def test_setup_records_count_plies_from_their_first_move(ok, game):
    fen = "4k3/8/8/8/8/8/4P3/4K3 b - - 3 40"
    out = ok("extract", game(fen=fen, moves=["e8d7", "e2e4", "d7e6"], complete=False),
             "--from-ply", 1, "--to-ply", 2)
    assert out["initial_fen"] == "8/3k4/8/8/8/8/4P3/4K3 w - - 4 41"
    assert out["moves"] == ["e2e4"]


def test_pgn_output_is_a_setup_game(cli, game):
    result = cli("extract", game(moves=RUY, game_id="ruy"), "--from-ply", 0, "--to-ply", 2,
                 "--pgn")
    assert result.returncode == 0, result.stderr
    assert result.stdout == (
        '[Event "?"]\n[Site "ruy@0-2"]\n[Date "????.??.??"]\n[Round "?"]\n'
        '[White "?"]\n[Black "?"]\n[Result "*"]\n[SetUp "1"]\n'
        f'[FEN "{START_FEN}"]\n\n1. e4 e5 *\n')
    result = cli("extract", game(moves=RUY, game_id="ruy"), "--from-ply", 1, "--to-ply", 4,
                 "--pgn")
    assert result.returncode == 0, result.stderr
    assert result.stdout.endswith("\n\n1... e5 2. Nf3 Nc6 *\n")
    assert '[Site "ruy@1-4"]' in result.stdout and '[SetUp "1"]' in result.stdout


def test_pgn_output_round_trips_through_import(ok, cli, game, tmp_path):
    out = ok("extract", game(moves=RUY, game_id="ruy"), "--from-ply", 3)
    result = cli("extract", game(moves=RUY, game_id="ruy"), "--from-ply", 3, "--pgn")
    path = tmp_path / "extract.pgn"
    path.write_text(result.stdout)
    back = ok("import-pgn", path)
    assert (back["game_id"], back["initial_fen"], back["history_complete"], back["moves"]) == (
        out["game_id"], out["initial_fen"], False, out["moves"])


def test_out_of_bounds_ranges_are_refused(cli, game):
    path = game(moves=RUY)
    for args in (("--from-ply", 4, "--to-ply", 3), ("--from-ply", 0, "--to-ply", 7),
                 ("--from-ply", 7), ("--from-ply", 7, "--pgn")):
        result = cli("extract", path, *args)
        assert refused(result), args
        assert "ply range out of bounds" in result.stderr


def test_bad_arguments_and_records_are_refused(cli, game):
    assert refused(cli("extract", game(moves=RUY)))
    assert refused(cli("extract", game(moves=RUY), "--from-ply", -1))
    assert refused(cli("extract", game(moves=["e2e4", "e2e4"]), "--from-ply", 0))
