"""K11: `perft-suite <file.epd> [--max-depth N]` checks positions against published perft counts."""
import json

from conftest import START_FEN, refused

START4 = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -"
KIWIPETE = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -"
POS3 = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"
POS5 = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8"
REPORT_KEYS = {"positions", "checked", "passed", "failed", "skipped", "results"}


def suite(cli, tmp_path, text, *args):
    path = tmp_path / "suite.epd"
    path.write_text(text)
    result = cli("perft-suite", path, *args)
    return result, (json.loads(result.stdout) if result.stdout.strip() else None)


def test_start_position_exact(cli, tmp_path):
    result, out = suite(cli, tmp_path, f"{START4} ;D1 20 ;D2 400\n")
    assert result.returncode == 0, result.stderr
    assert out == {
        "positions": 1, "checked": 2, "passed": 2, "failed": 0, "skipped": 0,
        "results": [
            {"line": 1, "fen": START_FEN, "depth": 1, "expected": 20, "nodes": 20, "pass": True},
            {"line": 1, "fen": START_FEN, "depth": 2, "expected": 400, "nodes": 400, "pass": True},
        ],
    }


def test_published_positions_pass(cli, tmp_path):
    text = (f"{KIWIPETE} ;D1 48 ;D2 2039 ;D3 97862\n{POS3} ;D1 14 ;D2 191 ;D3 2812\n"
            f"{POS5} ;D1 44 ;D2 1486 ;D3 62379\n")
    result, out = suite(cli, tmp_path, text)
    assert result.returncode == 0, result.stderr
    assert (out["positions"], out["checked"], out["passed"], out["failed"]) == (3, 9, 9, 0)
    assert [r["fen"] for r in out["results"][::3]] == [KIWIPETE + " 0 1", POS3, POS5]


def test_line_numbers_and_entry_order(cli, tmp_path):
    text = f"# perft suite\n\n   {START4} ;D2 400 ;D1 20   \n# end\n{POS3} ;D1 14\n"
    result, out = suite(cli, tmp_path, text)
    assert result.returncode == 0, result.stderr
    assert [(r["line"], r["depth"]) for r in out["results"]] == [(3, 2), (3, 1), (5, 1)]
    assert out["positions"] == 2


def test_wrong_count_fails_after_reporting(cli, tmp_path):
    result, out = suite(cli, tmp_path, f"{START4} ;D1 20 ;D2 401\n{POS3} ;D1 14\n")
    assert result.returncode != 0
    assert "perft suite failed: 1 of 3" in result.stderr
    assert set(out) == REPORT_KEYS
    assert (out["checked"], out["passed"], out["failed"]) == (3, 2, 1)
    bad = out["results"][1]
    assert (bad["expected"], bad["nodes"], bad["pass"]) == (401, 400, False)


def test_deeper_entries_are_skipped(cli, tmp_path):
    text = f"{START4} ;D1 20 ;D2 400 ;D3 8902\n8/8/8/8/8/8/8/K6k w - - ;D7 1 ;D1 3\n"
    result, out = suite(cli, tmp_path, text, "--max-depth", 2)
    assert result.returncode == 0, result.stderr
    assert (out["checked"], out["skipped"]) == (3, 2)
    assert [r["depth"] for r in out["results"]] == [1, 2, 1]
    # The default limit is perft's maximum depth: the D7 entry (wrong on purpose) is skipped.
    result, out = suite(cli, tmp_path, "8/8/8/8/8/8/8/K6k w - - ;D7 1 ;D1 3\n")
    assert result.returncode == 0, result.stderr
    assert (out["checked"], out["skipped"], out["failed"]) == (1, 1, 0)


def test_max_depth_bounds(cli, tmp_path):
    for depth in (0, 7):
        result, _ = suite(cli, tmp_path, f"{START4} ;D1 20\n", "--max-depth", depth)
        assert refused(result)
        assert "--max-depth must be between 1 and 6" in result.stderr


def test_adjudicated_positions_are_still_counted(cli, tmp_path):
    text = ("8/8/8/8/8/8/8/K6k w - - ;D1 3 ;D2 9\n"
            "8/8/8/8/8/8/8/KB5k b - - ;D1 3\n"
            "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3 ;D1 0 ;D2 0\n")
    result, out = suite(cli, tmp_path, text)
    assert result.returncode == 0, result.stderr
    assert [r["nodes"] for r in out["results"]] == [3, 9, 3, 0, 0]


def test_malformed_lines_are_refused(cli, tmp_path):
    cases = {
        f"{START4} ;D1\n": 1,
        f"# header\n{START4}\n": 2,
        f"{START4} ;D0 1\n": 1,
        f"{START4} ;D1 20 ;D1 20\n": 1,
        f"{START4} ;D1 twenty\n": 1,
        f"{START4} ;X1 20\n": 1,
        f"{START4} ;D1 20\nrnbqkbnr/pppppppp/8/8 w - ;D1 20\n": 2,
        f"{START4} ;D1 20\n8/8/8/8/8/8/8/8 w - - ;D1 0\n": 2,
    }
    for text, line in cases.items():
        result, _ = suite(cli, tmp_path, text)
        assert refused(result), text
        assert f"line {line}: " in result.stderr, (text, result.stderr)


def test_empty_suites_and_missing_files_are_refused(cli, tmp_path):
    for text in ("", "# nothing here\n\n"):
        result, _ = suite(cli, tmp_path, text)
        assert refused(result)
    assert refused(cli("perft-suite", tmp_path / "missing.epd"))
