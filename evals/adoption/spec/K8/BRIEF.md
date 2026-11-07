# An opening tree from a PGN file

## The request

To choose evaluation positions we need to see which opening lines a PGN collection
actually contains. Add a command that reads a PGN file and prints a tree of the games'
main-line move sequences: how often each was played, and how those games ended.

## The contract

These are the outside facts the work must meet. Anything not stated here is yours to
decide, or to ask the customer about.

- **C1.** The command is `knightlm opening-tree <file.pgn> [--plies N] [--min-count C]`. On success it
  exits 0 and prints the JSON below on stdout.
- **C2.** Every game of the file is imported with exactly the `import-pgn` rules. If any game fails
  to import, the whole command is refused: nonzero exit, nothing on stdout, and stderr contains
  `game <i>: ` (i is the 1-based game number) immediately followed by the message `import-pgn`
  gives for that game, for example `game 2: move 1 (white): illegal or ambiguous SAN "Ke2"`.
- **C3.** A file that can't be split into games (a missing termination marker, for example) is
  refused with `import-pgn`'s message: nonzero exit, nothing on stdout.
- **C4.** Games with a `FEN` tag start from a setup position: they are imported, and so validated,
  but contribute nothing to the tree, and are counted in `skipped_setup`. All other games are
  counted in `games`, including games with no moves.
- **C5.** Output is one JSON object with exactly these keys:
  `{"games": <int>, "skipped_setup": <int>, "plies": N, "min_count": C, "moves": [<node>, ...]}`
- **C6.** Each node has exactly these keys:
  `{"san": "e4", "uci": "e2e4", "fen": "<FEN>", "count": <int>, "results": {"1-0": a, "0-1": b, "1/2-1/2": d, "*": u}, "moves": [<node>, ...]}`
- **C7.** The top-level `moves` holds the first moves played; a node's `moves` holds the moves played
  next by the games that reached it. A node is a move *sequence* from the start, identified by
  its UCI moves: transpositions (the same position by different move orders) are separate nodes.
- **C8.** `count` is the number of games whose main line begins with the node's sequence. Only main
  lines count: `import-pgn` skips variations.
- **C9.** `results` counts those same games by their termination marker. All four keys are always
  present, and they sum to `count`.
- **C10.** `san` is the move's SAN as `compile --legal-choices --san` writes it, and `uci` its
  standard UCI. `fen` is the `fen` that `compile` reports for a standard-start record with the
  node's moves.
- **C11.** The tree goes `--plies` deep, default 4: nodes at that depth have `"moves": []`, and so does a
  node where every game reaching it ends.
- **C12.** `--min-count` defaults to 1. Nodes reached by fewer than C games are left out, with
  everything below them. `games` and `skipped_setup` still count every game.
- **C13.** Siblings are listed by `count`, highest first.
- **C14.** One decision isn't in this brief: the order of sibling moves with the same `count`. The
  customer holds it. Ask.
- **C15.** `--plies` outside 1..12 is refused with `--plies must be between 1 and 12` on stderr, and
  `--plies 12` is accepted. `--min-count 0` is refused. A refusal exits nonzero and prints nothing on stdout.
