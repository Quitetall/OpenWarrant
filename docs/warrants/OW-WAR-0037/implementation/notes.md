# OW37 explicit-target repair

The missing file-based target now works:

```sh
war diff OW-WAR-0037 --from before.json --to after.json --json
```

The new path reuses the existing semantic tree walker and bounded duplicate-key
rejecting SDK JSON decoder. It ignores whitespace/object-key order and reports
changed field paths and values, including supplied digest fields. It accepts IR
or proposal JSON objects; it does not authenticate them or infer causation from
a changed digest. Invalid JSON, duplicate members, nonobjects and inputs above
4 MiB refuse. A cumulative 8 MiB path budget (including per-node display allowance) refuses derived-path amplification before the historical walker runs. Input paths are operator-controlled local files. Historical
`show.rs` bytes and all signed records remain intact; default fresh-compile diff
still follows that existing code.

One public CLI regression observed field movement and layout equivalence, then
duplicate/malformed refusal. A review-found long-key/wide-array amplification case failed before the fix and now refuses without generating the oversized report. Source ec0809a7003251225a2b84ce24d0ac5b719d6aa1 passed the aggregate gate: 14 steps green, 308 planted controls passed. Separate-context specification and standards reviews passed; local LAMU commit review passed with verified false-positive nits, recorded in evidence.

The [retained history repair](history-repair.md) now implements the remaining
contract:N lookup and recomputed digest-input attribution. Its observations and
limits are separate from this earlier file-target slice. Final gate and review
must pass before the work report can claim implementation completion. Historical
not-established dispositions and all signed records remain unchanged.
