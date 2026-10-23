"""K3: `compile --legal-choices --san` adds `legal_moves_san`."""
from conftest import START_FEN


def refused(result):
    return result.returncode != 0 and result.stdout.strip() == ""


def san_map(ok, path):
    packet = ok("compile", path, "--legal-choices", "--san")
    assert len(packet["legal_moves_san"]) == len(packet["legal_moves"])
    return dict(zip(packet["legal_moves"], packet["legal_moves_san"]))


def test_start_position_san_is_aligned_with_uci_order(ok, game):
    packet = ok("compile", game(), "--legal-choices", "--san")
    assert packet["legal_moves"][:4] == ["a2a3", "a2a4", "b1a3", "b1c3"]
    assert packet["legal_moves_san"] == [
        "a3", "a4", "Na3", "Nc3", "b3", "b4", "c3", "c4", "d3", "d4",
        "e3", "e4", "f3", "f4", "Nf3", "Nh3", "g3", "g4", "h3", "h4",
    ]


def test_file_disambiguation(ok, game):
    san = san_map(ok, game("4k3/8/8/8/8/8/8/1N1K1N2 w - - 0 1"))
    assert san["b1d2"] == "Nbd2" and san["f1d2"] == "Nfd2"
    assert san["d1d2"] == "Kd2"
    assert san["b1c3"] == "Nc3" and san["f1g3"] == "Ng3"


def test_rank_disambiguation_and_check(ok, game):
    san = san_map(ok, game("4k3/8/8/R7/8/8/8/R3K3 w - - 0 1"))
    assert san["a1a3"] == "R1a3" and san["a5a3"] == "R5a3"
    assert san["a1b1"] == "Rb1"
    assert san["a5a8"] == "Ra8+"


def test_file_and_rank_disambiguation(ok, game):
    san = san_map(ok, game("4k3/8/8/8/8/Q7/8/Q1Q1K3 w - - 0 1"))
    assert san["a1b2"] == "Qa1b2"
    assert san["c1b2"] == "Qcb2"
    assert san["a3b2"] == "Q3b2"


def test_pinned_piece_does_not_cause_disambiguation(ok, game):
    san = san_map(ok, game("k3r3/8/8/1N6/8/8/4N3/4K3 w - - 0 1"))
    assert san["b5c3"] == "Nc3" and san["b5d4"] == "Nd4"
    assert not any(m.startswith("e2") for m in san)


def test_captures_en_passant_and_promotions(ok, game):
    san = san_map(ok, game("r3k3/1P6/8/3pP3/8/8/8/4K3 w - d6 0 2"))
    assert san["e5d6"] == "exd6"
    assert san["e5e6"] == "e6"
    assert san["b7a8q"] == "bxa8=Q+"
    assert san["b7b8q"] == "b8=Q+"
    assert san["b7b8n"] == "b8=N"
    assert san["b7a8r"] == "bxa8=R+"


def test_castling(ok, game):
    san = san_map(ok, game("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"))
    assert san["e1g1"] == "O-O" and san["e1c1"] == "O-O-O"
    assert san["a1a8"] == "Rxa8+"
    black = san_map(ok, game("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1", moves=["a1b1"]))
    assert black["e8g8"] == "O-O" and black["e8c8"] == "O-O-O"


def test_checkmate_suffix(ok, game):
    san = san_map(ok, game(moves=["f2f3", "e7e5", "g2g4"]))
    assert san["d8h4"] == "Qh4#"
    scholar = san_map(ok, game(moves=["e2e4", "e7e5", "d1h5", "b8c6", "f1c4", "g8f6"]))
    assert scholar["h5f7"] == "Qxf7#"
    assert scholar["c4f7"] == "Bxf7+"


def test_terminal_position_lists_no_moves(ok, game):
    packet = ok("compile", game(moves=["f2f3", "e7e5", "g2g4", "d8h4"]), "--legal-choices", "--san")
    assert packet["legal_moves"] == [] and packet["legal_moves_san"] == []


def test_field_absent_without_flag(ok, game):
    packet = ok("compile", game(), "--legal-choices")
    assert "legal_moves_san" not in packet
    assert "legal_moves_san" not in ok("compile", game())


def test_san_requires_legal_choices(cli, game):
    assert refused(cli("compile", game(), "--san"))
