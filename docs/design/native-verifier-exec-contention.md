# Native verifier startup contention

A recorded Katana mapping control failed before its verifier could run:
`Unavailable("Text file busy (os error 26)")`. This was not an observed acceptance
of the invalid `assurance_granted` field. The original process transport returned
the first failed exec immediately.

The shared transport now retries only the operating system's `ETXTBSY` launch
error. One existing configured deadline covers those retries and the subsequent
program execution. A started program is never repeated. Other spawn failures
retain their unavailable result. Persistent contention reaches the startup
deadline, retains its busy-error diagnostic and returns UNKNOWN; it produces no
protocol verdict. Stream limits, process-group cleanup and protocol validation
are unchanged. Existing termination/reaping grace is separate from execution time.

Four Linux controls hold a writer on an owned executable to reproduce the same
kernel error. They exercise release followed by actual execution and preserved
stdin, permanent contention, one deadline across retry and running program, and
unchanged missing-executable failure. This proves bounded recovery of that error
class, not every possible blocking kernel operation or startup failure.

The writer responsible for the earlier full-suite occurrence remains unidentified.
The tests do not prove production collector identity, confinement, charges, native
receipt authenticity, revocation fencing or macOS behavior. No recorded authority,
independence requirement or expected mapping rejection is weakened.
