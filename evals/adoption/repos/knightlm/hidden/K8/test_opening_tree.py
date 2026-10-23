"""K8: `opening-tree <file.pgn> [--plies N] [--min-count C]` counts main-line move sequences."""
import json

from conftest import START_FEN, refused

TREE_KEYS = {"games", "skipped_setup", "plies", "min_count", "moves"}
NODE_KEYS = {"san", "uci", "fen", "count", "results", "moves"}
NO_RESULTS = {"1-0": 0, "0-1": 0, "1/2-1/2": 0, "*": 0}


def pgn(*movetexts):
    """A PGN file of tagless games, one per movetext (each ends with its marker)."""
    return "\n".join(f'[Event "t"]\n\n{text}\n' for text in movetexts)


def tree(ok, tmp_path, text, *args, name="games.pgn"):
    path = tmp_path / name
    path.write_text(text)
    return ok("opening-tree", path, *args)


def sans(nodes):
    return [node["san"] for node in nodes]


def results(**counts):
    out = dict(NO_RESULTS)
    out.update({{"w": "1-0", "b": "0-1", "d": "1/2-1/2", "u": "*"}[k]: v for k, v in counts.items()})
    return out


def walk(nodes):
    for node in nodes:
        yield node
        yield from walk(node["moves"])


def test_exact_tree(ok, tmp_path):
    out = tree(ok, tmp_path, pgn("1. e4 e5 2. Nf3 Nc6 1-0", "1. e4 e5 2. Bc4 0-1",
                                 "1. d4 d5 1/2-1/2"), "--plies", 2)
    assert out == {
        "games": 3, "skipped_setup": 0, "plies": 2, "min_count": 1,
        "moves": [
            {"san": "e4", "uci": "e2e4",
             "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
             "count": 2, "results": results(w=1, b=1),
             "moves": [{"san": "e5", "uci": "e7e5",
                        "fen": "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2",
                        "count": 2, "results": results(w=1, b=1), "moves": []}]},
            {"san": "d4", "uci": "d2d4",
             "fen": "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3 0 1",
             "count": 1, "results": results(d=1),
             "moves": [{"san": "d5", "uci": "d7d5",
                        "fen": "rnbqkbnr/ppp1pppp/8/3p4/3P4/8/PPP1PPPP/RNBQKBNR w KQkq d6 0 2",
                        "count": 1, "results": results(d=1), "moves": []}]},
        ],
    }


def test_default_depth_is_four_plies(ok, tmp_path):
    out = tree(ok, tmp_path, pgn("1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 *"))
    assert out["plies"] == 4 and out["min_count"] == 1
    line = [out["moves"][0]]
    while line[-1]["moves"]:
        assert len(line[-1]["moves"]) == 1
        line.append(line[-1]["moves"][0])
    assert [n["san"] for n in line] == ["e4", "e5", "Nf3", "Nc6"]
    assert line[-1]["moves"] == []
    assert all(set(n) == NODE_KEYS for n in walk(out["moves"]))


def test_more_frequent_moves_come_first(ok, tmp_path):
    out = tree(ok, tmp_path, pgn("1. c4 *", "1. e4 *", "1. e4 *", "1. Nf3 *", "1. Nf3 *",
                                 "1. Nf3 *"), "--plies", 1)
    assert [(n["san"], n["count"]) for n in out["moves"]] == [("Nf3", 3), ("e4", 2), ("c4", 1)]


def test_ties_follow_the_answer(ok, tmp_path):
    out = tree(ok, tmp_path, pgn("1. e4 e5 *", "1. d4 d5 *", "1. e4 c5 *", "1. c4 e5 *",
                                 "1. d4 Nf6 *", "1. e4 d5 *"), "--plies", 2)
    assert sans(out["moves"]) == ["e4", "d4", "c4"]
    e4, d4 = out["moves"][0], out["moves"][1]
    assert sans(e4["moves"]) == ["e5", "c5", "d5"]
    assert sans(d4["moves"]) == ["d5", "Nf6"]


def test_ties_after_counting(ok, tmp_path):
    out = tree(ok, tmp_path, pgn("1. h4 *", "1. g4 *", "1. g4 *", "1. h4 *", "1. a4 *",
                                 "1. b4 *", "1. a3 *"), "--plies", 1)
    assert sans(out["moves"]) == ["h4", "g4", "a4", "b4", "a3"]


def test_results_count_termination_markers(ok, tmp_path):
    out = tree(ok, tmp_path, pgn("1. e4 e5 1-0", "1. e4 c5 0-1", "1. e4 e5 1/2-1/2",
                                 "1. e4 e5 *", "1. e4 c5 1-0"), "--plies", 2)
    e4 = out["moves"][0]
    assert e4["count"] == 5 and e4["results"] == results(w=2, b=1, d=1, u=1)
    assert [(n["san"], n["results"]) for n in e4["moves"]] == [
        ("e5", results(w=1, d=1, u=1)), ("c5", results(w=1, b=1))]
    for node in walk(out["moves"]):
        assert sum(node["results"].values()) == node["count"]


def test_setup_and_short_games(ok, tmp_path):
    setup = '[Event "s"]\n[SetUp "1"]\n[FEN "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1"]\n\n1. e4 Kd7 *\n'
    text = pgn("1. e4 *", "*", "1. e4 e5 2. Nf3 1-0") + "\n" + setup
    out = tree(ok, tmp_path, text, "--plies", 3)
    assert out["games"] == 3 and out["skipped_setup"] == 1
    assert [(n["san"], n["count"]) for n in out["moves"]] == [("e4", 2)]
    e5 = out["moves"][0]["moves"][0]
    assert (e5["san"], e5["count"]) == ("e5", 1)
    assert sans(e5["moves"]) == ["Nf3"] and e5["moves"][0]["moves"] == []
    assert "Kd7" not in {n["san"] for n in walk(out["moves"])}


def test_transpositions_stay_separate_and_fens_match_compile(ok, game, tmp_path):
    out = tree(ok, tmp_path, pgn("1. Nf3 d5 2. d4 *", "1. d4 d5 2. Nf3 *"), "--plies", 3)
    assert sans(out["moves"]) == ["Nf3", "d4"]
    leaves = [n for n in walk(out["moves"]) if not n["moves"]]
    assert len(leaves) == 2
    for path in (["g1f3", "d7d5", "d2d4"], ["d2d4", "d7d5", "g1f3"]):
        nodes, fen = out["moves"], None
        for uci in path:
            node = next(n for n in nodes if n["uci"] == uci)
            nodes, fen = node["moves"], node["fen"]
        assert fen == ok("compile", game(moves=path))["fen"]


def test_min_count_prunes_rare_lines(ok, tmp_path):
    text = pgn("1. e4 e5 *", "1. e4 e5 *", "1. e4 c5 *", "1. d4 d5 *")
    out = tree(ok, tmp_path, text, "--plies", 2, "--min-count", 2)
    assert out["min_count"] == 2 and out["games"] == 4
    assert [(n["san"], n["count"]) for n in out["moves"]] == [("e4", 3)]
    assert [(n["san"], n["count"]) for n in out["moves"][0]["moves"]] == [("e5", 2)]
    assert tree(ok, tmp_path, text, "--plies", 2, "--min-count", 5)["moves"] == []


def test_depth_and_count_bounds(cli, ok, tmp_path):
    path = tmp_path / "g.pgn"
    path.write_text(pgn("1. e4 e5 *"))
    for plies in (0, 13):
        result = cli("opening-tree", path, "--plies", plies)
        assert refused(result)
        assert "--plies must be between 1 and 12" in result.stderr
    result = cli("opening-tree", path, "--min-count", 0)
    assert refused(result)
    assert ok("opening-tree", path, "--plies", 12)["plies"] == 12


def test_one_bad_game_refuses_the_file(cli, tmp_path):
    path = tmp_path / "bad.pgn"
    path.write_text(pgn("1. e4 e5 *", "1. Ke2 *"))
    result = cli("opening-tree", path)
    assert refused(result)
    assert 'game 2: move 1 (white): illegal or ambiguous SAN "Ke2"' in result.stderr
    path.write_text('[Result "1-0"]\n\n1. e4 e5 0-1\n')
    result = cli("opening-tree", path)
    assert refused(result)
    assert "game 1: " in result.stderr and "disagrees with termination marker" in result.stderr
    path.write_text("1. e4 e5 2. Nf3\n")
    result = cli("opening-tree", path)
    assert refused(result)
    assert "missing game termination marker" in result.stderr
