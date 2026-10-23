# Context budget and cache observations

`run.py --provider-root <LAMU-Rust-workspace> --output <new-file>` checks the
shared profile and runs the three original provider tests.

`check_observations.py --war <SDK-CLI> --transcript <probe-stdout>
--record-dir <new-directory>` checks the additional external caller's observations
and compares its basis keys with the SDK's domain-separated digest operation.
The caller source and execution identity are retained with W82's evidence.
It is a disposable external test caller, not an OpenWarrant dependency or a
change to provider logic.

The cases cover inclusive final-entry accounting, Unicode bytes, framing, all
eleven operational limits, basis changes, rule reactivation, changed source,
failed/oversized replacement, tiny capacity and caller-held results after clear.
Accounting is a declared byte estimate, not a tokenizer or spend meter. Capacity
covers retained logical payload, not allocator overhead or caller-held Arc memory.
These observations do not establish execution permission or qualification.
