"""K9: `audit-pgn <file.pgn> [--only-failures]` audits every game of a PGN file."""
import json

from conftest import refused

ENTRY_KEYS = {"index", "game_id", "termination", "length", "final_fen", "computed_result",
              "terminal_reason", "consistent", "claimable_draw", "error"}
SUMMARY_KEYS = {"games", "errors", "inconsistent", "claimable_draws", "total_length"}
SHUFFLE = "1. Nf3 Nf6 2. Ng1 Ng8 3. Nf3 Nf6 4. Ng1 Ng8"


def game(movetext, **tags):
    head = "".join(f'[{k} "{v}"]\n' for k, v in tags.items())
    return f"{head or '[Event \"t\"]' + chr(10)}\n{movetext}\n"


def audit(cli, tmp_path, *games, args=(), name="games.pgn"):
    path = tmp_path / name
    path.write_text("\n".join(games))
    result = cli("audit-pgn", path, *args)
    return result, (json.loads(result.stdout) if result.stdout.strip() else None)


def test_clean_game_exact(cli, tmp_path):
    result, out = audit(cli, tmp_path, game("1. f3 e5 2. g4 Qh4# 0-1", Site="fool"))
    assert result.returncode == 0, result.stderr
    assert out == {
        "games": [{
            "index": 1, "game_id": "fool", "termination": "0-1", "length": 2,
            "final_fen": "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3",
            "computed_result": "0-1", "terminal_reason": "checkmate", "consistent": True,
            "claimable_draw": False, "error": None,
        }],
        "summary": {"games": 1, "errors": 0, "inconsistent": 0, "claimable_draws": 0,
                    "total_length": 2},
    }


def test_length_follows_the_answer(cli, tmp_path):
    setup = game("40... Kd7 41. e4 Ke6 *", FEN="4k3/8/8/8/8/8/4P3/4K3 b - - 3 40")
    result, out = audit(cli, tmp_path, game("1. e4 e5 2. Nf3 *"), game("1. e4 *"), game("*"),
                        setup, game("1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 *"))
    assert result.returncode == 0, result.stderr
    assert [g["length"] for g in out["games"]] == [2, 1, 0, 2, 3]
    assert out["summary"]["total_length"] == 8


def test_undetermined_positions_accept_any_marker(cli, tmp_path):
    result, out = audit(cli, tmp_path, game("1. e4 e5 1-0"), game("1. d4 0-1"),
                        game("1. c4 1/2-1/2"), game("1. Nf3 *"))
    assert result.returncode == 0, result.stderr
    assert [g["computed_result"] for g in out["games"]] == ["*"] * 4
    assert all(g["consistent"] is True for g in out["games"])
    assert all(g["terminal_reason"] is None for g in out["games"])


def test_wrong_marker_after_mate_is_inconsistent(cli, tmp_path):
    result, out = audit(cli, tmp_path, game("1. f3 e5 2. g4 Qh4# 1/2-1/2"),
                        game("1. e4 *"))
    assert result.returncode != 0
    assert "audit failed: 0 errors, 1 inconsistent" in result.stderr
    first = out["games"][0]
    assert (first["computed_result"], first["consistent"], first["error"]) == ("0-1", False, None)
    assert out["summary"]["inconsistent"] == 1 and out["summary"]["errors"] == 0


def test_draw_positions_fix_the_marker(cli, tmp_path):
    stalemate = "7k/4Q3/6K1/8/8/8/8/8 w - - 0 1"
    dead = "4kb2/8/8/8/8/8/3r4/2B1K3 w - - 0 1"
    result, out = audit(cli, tmp_path, game("1. Qf7 1/2-1/2", FEN=stalemate),
                        game("1. Qf7 1-0", FEN=stalemate),
                        game("1. Kxd2 1/2-1/2", FEN=dead), game("1. Kxd2 *", FEN=dead))
    assert result.returncode != 0
    assert "audit failed: 0 errors, 2 inconsistent" in result.stderr
    rows = [(g["computed_result"], g["terminal_reason"], g["consistent"]) for g in out["games"]]
    assert rows == [("1/2-1/2", "stalemate", True), ("1/2-1/2", "stalemate", False),
                    ("1/2-1/2", "proven_dead_position", True),
                    ("1/2-1/2", "proven_dead_position", False)]


def test_claimable_draws(cli, tmp_path):
    result, out = audit(cli, tmp_path, game(SHUFFLE + " *"),
                        game(SHUFFLE + " *", FEN="rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"),
                        game("60. Ra2 *", FEN="4k3/8/8/8/8/8/8/R3K3 w - - 99 60"),
                        game("1. e4 e5 *"))
    assert result.returncode == 0, result.stderr
    assert [g["claimable_draw"] for g in out["games"]] == [True, False, True, False]
    assert out["summary"]["claimable_draws"] == 2
    assert all(g["terminal_reason"] is None and g["consistent"] for g in out["games"])


def test_bad_games_are_reported_and_the_audit_goes_on(cli, tmp_path):
    result, out = audit(cli, tmp_path, game("1. e4 e5 *", Site="ok-1"),
                        game("1. e4 e5 2. Ke3 *", Site="bad-san"),
                        game("1. e4 e5 0-1", Result="1-0"), game("1. d4 *"))
    assert result.returncode != 0
    assert "audit failed: 2 errors, 0 inconsistent" in result.stderr
    games = out["games"]
    assert [set(g) for g in games] == [ENTRY_KEYS] * 4
    assert [g["index"] for g in games] == [1, 2, 3, 4]
    assert games[0]["error"] is None and games[3]["error"] is None
    assert 'move 2 (white): illegal or ambiguous SAN "Ke3"' in games[1]["error"]
    assert "disagrees with termination marker" in games[2]["error"]
    for bad in games[1:3]:
        assert {k: bad[k] for k in ENTRY_KEYS - {"index", "termination", "error"}} == {
            k: None for k in ENTRY_KEYS - {"index", "termination", "error"}}
    assert [g["termination"] for g in games] == ["*", "*", "0-1", "*"]
    assert out["summary"] == {"games": 4, "errors": 2, "inconsistent": 0,
                              "claimable_draws": 0, "total_length": 2}


def test_game_ids_and_final_fen_match_import(cli, ok, tmp_path):
    text = [game("1. e4 e5 2. Nf3 Nc6 *", Site="https://example.org/g1"),
            game("1. d4 d5 2. c4 *", Site="?")]
    result, out = audit(cli, tmp_path, *text, name="ids.pgn")
    assert result.returncode == 0, result.stderr
    assert [g["game_id"] for g in out["games"]] == ["https://example.org/g1", "ids.pgn#2"]
    pgn = tmp_path / "ids.pgn"
    for entry in out["games"]:
        record = tmp_path / f"r{entry['index']}.json"
        record.write_text(json.dumps(ok("import-pgn", pgn, "--game", entry["index"])))
        assert entry["final_fen"] == ok("compile", record)["fen"]


def test_only_failures(cli, tmp_path):
    games = [game("1. e4 *"), game("1. f3 e5 2. g4 Qh4# 1-0"), game("1. Ke2 *"), game("1. d4 *")]
    result, out = audit(cli, tmp_path, *games, args=("--only-failures",))
    assert result.returncode != 0
    assert [g["index"] for g in out["games"]] == [2, 3]
    assert out["summary"] == {"games": 4, "errors": 1, "inconsistent": 1,
                              "claimable_draws": 0, "total_length": 4}
    result, out = audit(cli, tmp_path, game("1. e4 *"), args=("--only-failures",))
    assert result.returncode == 0 and out["games"] == []
    assert set(out["summary"]) == SUMMARY_KEYS


def test_unsplittable_files_are_refused(cli, tmp_path):
    for text in ("1. e4 e5 2. Nf3\n", "", "1. e4 (1. d4 e5 *\n"):
        path = tmp_path / "broken.pgn"
        path.write_text(text)
        assert refused(cli("audit-pgn", path))
    assert refused(cli("audit-pgn", tmp_path / "missing.pgn"))
