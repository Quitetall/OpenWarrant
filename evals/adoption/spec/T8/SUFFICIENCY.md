# Sufficiency check: T8

Status: for the owner's review. Written by the session that built the harness, which has seen the
hidden tests. PROTOCOL-spec.md also asks for a strong reader in a fresh session: that reading is
still owed. `run.py spec-check` proves only the mechanical half: that every assertion has a row,
every clause exists and every clause is needed. Whether a clause really determines an assertion
is a judgment.

Status: `contract` means the brief alone determines it. `customer` means it also needs the answer
that C8 tells the author to ask for, so a spec that never asks fails it. See "Design problem".

| assertion | what it checks | determined by | status |
|---|---|---|---|
| `test_three_months_carry_the_remainder_forward.1` | exit 0 | C1 | contract |
| `test_three_months_carry_the_remainder_forward.2` | full stdout: 3 blocks, blank lines, Budget 400, 650, 550 | C2, C3, C4, C5, C6, C7, C8 | customer |
| `test_overspending_reduces_the_next_month.1` | exit 0 | C1 | contract |
| `test_overspending_reduces_the_next_month.2` | January row: Budget 100, Remaining -80 | C3, C6, C7 | contract |
| `test_overspending_reduces_the_next_month.3` | February row: Budget 20 | C6, C8 | customer |
| `test_partial_first_and_last_months.1` | exit 0 | C1 | contract |
| `test_partial_first_and_last_months.2` | partial headers, weekly rule, Budget 30, 58, 68, and March's 99 left out of the period | C2, C3, C6, C7, C8 | customer |
| `test_one_month_gives_one_block.1` | exit 0 | C1 | contract |
| `test_one_month_gives_one_block.2` | one block; no occurrence inside it, so Budget 0.00 and the row still shows | C2, C3, C5, C6, C7 | contract |
| `test_every_block_lists_every_budgeted_account.1` | exit 0 | C1 | contract |
| `test_every_block_lists_every_budgeted_account.2` | January: header, and Insurance at 0.00 with no occurrence | C3, C5, C7 | contract |
| `test_every_block_lists_every_budgeted_account.3` | February: Food 800, Insurance 300 | C7, C8 | customer |
| `test_every_block_lists_every_budgeted_account.4` | March: Food 1200, Insurance 300, Remaining 50 | C6, C7, C8 | customer |
| `test_carry_is_per_commodity.1` | exit 0 | C1 | contract |
| `test_carry_is_per_commodity.2` | rows per commodity, default commodity first, `0.00 EUR` format, February's figures | C3, C5, C7, C8 | customer |
| `test_without_monthly_budget_is_unchanged.1` | exit 0 | C1 | contract |
| `test_without_monthly_budget_is_unchanged.2` | one table, as today | C1 | contract |
| `test_from_after_until_is_refused.1` | exit 2 | C9 | contract |
| `test_from_after_until_is_refused.2` | nothing on stdout | C9 | contract |
| `test_from_after_until_is_refused.3` | exact stderr | C9 | contract |
| `test_monthly_still_requires_both_dates.1` | exit 2 | C9 | contract |
| `test_monthly_still_requires_both_dates.2` | nothing on stdout | C9 | contract |

## Exclusions

None. Every assertion is determined by the contract, or by the contract and the customer's answer.

## Not assertions, but relied on

- `row()` in the tests fixes the column widths (30, 14, 14, 14). C3's "exactly today's `budget`
  format" covers this: the seed has T4's `render_budget`, and an author can read it.
- The first block takes nothing from an earlier one. `test_overspending_reduces_the_next_month.2` and
  `test_every_block_lists_every_budgeted_account.2` depend on this. They are marked `contract`, because
  every reading of a first block gives the same figures.

## Design problem: a third status

PROTOCOL-spec.md says an undetermined assertion goes into the contract or is excluded. It also says
that the customer answers from the item's `question` field. For T8 these collide: 6 of the 22
assertions, in 5 of the 9 tests, depend on the answer, and T8 passes only if all 9 do. In the
contract the customer would have nothing to answer. Excluded, T8 would shrink to 4 tests and lose the
item's hard part.

So the brief takes a third path. C8 is a sentence in the brief that says the decision exists and
who holds it, which is what the backlog's own "Open question" line does. The brief then determines
the assertion by reference, and the customer still answers only what is asked. The cost is that the
question becomes hard to miss, so "questions asked" measures less than it could. The owner may
prefer to delete C8. Then T8's result depends on whether an author notices the gap unprompted,
with a real risk that both authors fail it and the pilot's stop rule fires. Deleting C8 also needs
`run.py spec-check` to allow an unprompted customer row.
