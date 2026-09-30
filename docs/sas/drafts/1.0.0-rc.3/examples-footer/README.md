# RC.3 footer examples

These are unsigned authoring examples, separate from the immutable RC.2 fixtures.
Read [minimal.md](minimal.md), [signup.md](signup.md) and [architecture.md](architecture.md).
The minimal draft is valid but does not claim readiness or permission. These three
source files demonstrate new framing; they are not newly generated context packets.
Existing expected packets under `examples/` still bind the original RC.2 bytes.

`python3 ../test_footer.py` checks exact footer boundaries, LF/CRLF, title agreement,
compact dependency equivalence and malformed/framing refusal cases. This is a
small documentation witness. The production SDK must repeat these observations
through its public parse/validate and author/edit functions in OW-WAR-0075/0076.
