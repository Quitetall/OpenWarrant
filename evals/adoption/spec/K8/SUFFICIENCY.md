# Sufficiency check: K8

Status: for the owner's review. Written by the session that built the harness, which has seen the
hidden tests. PROTOCOL-spec.md also asks for a strong reader in a fresh session: that reading is
still owed. `run.py spec-check` proves only the mechanical half: that every assertion has a row,
every clause exists and every clause is needed. Whether a clause really determines an assertion
is a judgment.

Status: `contract` means the brief alone determines it. `customer` means it also needs the answer
that C14 tells the author to ask for. `ok.1` is the fixture behind every successful run.

| assertion | what it checks | determined by | status |
|---|---|---|---|
| `ok.1` | exit 0 on success, stdout parses as JSON | C1 | contract |
| `test_exact_tree.1` | the whole object, keys and values | C5, C6, C7, C8, C9, C10, C11, C13 | contract |
| `test_default_depth_is_four_plies.1` | `plies` 4 and `min_count` 1 by default | C5, C11, C12 | contract |
| `test_default_depth_is_four_plies.2` | the line stops at 4 plies, with SAN | C10, C11 | contract |
| `test_default_depth_is_four_plies.3` | the last node's `moves` is empty | C11 | contract |
| `test_default_depth_is_four_plies.4` | every node has exactly the keys | C6 | contract |
| `test_default_depth_is_four_plies.5` | one child per level | C7 | contract |
| `test_more_frequent_moves_come_first.1` | siblings by count, highest first | C8, C13 | contract |
| `test_ties_follow_the_answer.1` | tied first moves in the customer's order | C13, C14 | customer |
| `test_ties_follow_the_answer.2` | ties at the next level | C14 | customer |
| `test_ties_follow_the_answer.3` | ties under another parent | C14 | customer |
| `test_ties_after_counting.1` | the order holds after counts are compared | C13, C14 | customer |
| `test_results_count_termination_markers.1` | `count` and `results` for a mixed node | C8, C9 | contract |
| `test_results_count_termination_markers.2` | children's results, ordered by count | C9, C13 | contract |
| `test_results_count_termination_markers.3` | `results` sum to `count` at every node | C9 | contract |
| `test_setup_and_short_games.1` | `games` 3, including the empty game, `skipped_setup` 1 | C4 | contract |
| `test_setup_and_short_games.2` | a setup game contributes no node | C4, C8 | contract |
| `test_setup_and_short_games.3` | count under a shorter game | C8 | contract |
| `test_setup_and_short_games.4` | the branch ends with the game | C11 | contract |
| `test_setup_and_short_games.5` | a setup game's move appears nowhere | C4 | contract |
| `test_transpositions_stay_separate_and_fens_match_compile.1` | two tied first moves, in the customer's order | C13, C14 | customer |
| `test_transpositions_stay_separate_and_fens_match_compile.2` | transpositions are separate leaves | C7 | contract |
| `test_transpositions_stay_separate_and_fens_match_compile.3` | `fen` equals `compile`'s | C10 | contract |
| `test_min_count_prunes_rare_lines.1` | `min_count` echoed, `games` counts all | C5, C12 | contract |
| `test_min_count_prunes_rare_lines.2` | a rare first move is pruned | C12 | contract |
| `test_min_count_prunes_rare_lines.3` | a rare next move is pruned | C12 | contract |
| `test_min_count_prunes_rare_lines.4` | nothing reaches the count: `moves` is empty | C12 | contract |
| `test_depth_and_count_bounds.1` | `--min-count 0` refused | C15 | contract |
| `test_depth_and_count_bounds.2` | `--plies 12` accepted | C15 | contract |
| `test_depth_and_count_bounds.3` | `--plies` 0 and 13 refused | C15 | contract |
| `test_depth_and_count_bounds.4` | their exact message | C15 | contract |
| `test_one_bad_game_refuses_the_file.1` | illegal SAN refuses the file | C2 | contract |
| `test_one_bad_game_refuses_the_file.2` | `game 2: ` and `import-pgn`'s message | C2 | contract |
| `test_one_bad_game_refuses_the_file.3` | a Result tag that disagrees refuses the file | C2 | contract |
| `test_one_bad_game_refuses_the_file.4` | `game 1: ` and `import-pgn`'s message | C2 | contract |
| `test_one_bad_game_refuses_the_file.5` | a missing marker refuses the file | C3 | contract |
| `test_one_bad_game_refuses_the_file.6` | `import-pgn`'s message for it | C3 | contract |

## Exclusions

None. Every assertion is determined by the contract, or by the contract and the customer's answer.

## Not assertions, but relied on

- `conftest.py` builds with `cargo build --locked`, offline. A solution that adds a crate fails to
  build. The project facts, in every arm's instruction file, name `--locked`, and the seed's
  dependencies are enough.
- The binary is `<target>/debug/knightlm`, as it already is.
- `--min-count 0`'s message is in the backlog, but no test reads it, so the contract leaves it out.

## Design problem: a third status

As for T8, the tied order is neither contract nor exclusion: C14 points at the customer. 5 of the
37 assertions, in 3 of the 12 tests, depend on the answer. One of them,
`test_transpositions_stay_separate_and_fens_match_compile.1`, is a test about something else that
happens to tie, so a spec that gets everything but the order fails more than the two tie tests.
