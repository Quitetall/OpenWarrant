# Independent review — bounded OW37 explicit-target repair

Source: ec0809a7003251225a2b84ce24d0ac5b719d6aa1.

Separate-context specification reviewer: PASS for this bounded repair; public CLI regression independently passed. Historical selectors and digest attribution remain outside this repair.

Separate-context standards reviewer first found derived-path amplification: two 14 KiB inputs with an 8192-character parent key and 2048 changed leaves produced a 17.1 MiB report. After cumulative-path preflight, reviewer reran original probe and public test: PASS; probe refuses with `comparison path budget exceeds 8 MiB`.

LAMU review_commit source verdict: PASS WITH NITS, local Qwen backend. Findings verified before action: pre-open regular-file check and opened-handle check are deliberately separate; 4 MiB input and depth 64 bound arithmetic well below usize overflow; default target is always fresh compilation in show::diff. These three findings are false positives. No signed authority or whole-Warrant completion is asserted.
