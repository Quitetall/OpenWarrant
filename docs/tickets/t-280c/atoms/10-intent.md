# war next is slow (~7.5 s)

Ticket commands answer in 5-13 ms; war next takes ~7.5 s on this corpus (it dry-runs every pending act and rebuilds status). Profile it the way compile was (run under gdb, SIGINT the child) and cut it.

## Notes

- **2026-09-25 22:22 UTC, claude:** Before/after on this corpus, debug build, load avg ~13-20, 3 runs each: old 6.93/6.96/6.92 s, new 3.05/3.13/3.02 s. Left: status::build, frontier::run and resolution_cmd::request each assess() the same Warrants (364 assessments per run, ~1 s), and load_warrant is still called ~880 times (~0.75 s, mostly compat::newer_records walking each dir). Sharing those needs a read-session scoped to one command, not a global memo; not done here.
