# Local independent review status

Four actual local LAMU/Qwen reviews were ingested through `war evidence verify`.
The tool wrote the received records; no performer disposition was substituted.

- OBL-001: received **not_established**. The compact view omits the public
  probe's execution output and does not show its parsed results directly.
- OBL-002: received **not_established**. The structural gate is not a parser
  execution gate, and the visible view lacks the probe's refusal output.
- OBL-003: received **not_established**. Independent qualification and secure
  human acceptance of the exact result remain missing.
- OBL-004: received **established**, bounded to structural Warrant records and
  the actual structural gate receipt. This does not establish parser behavior.

The first two findings were checked against the actual view. D-033's archived
JSON contains the raw stdout, but its selected excerpts omit the `stdout`
field and the “22 document cases passed” line. Full archives retain those
bytes; their presence in an archive does not mean the model saw them. The
next implementation slice must expose actual parse/validation observations
and deliver the execution evidence in the review view. It must preserve these
received results and independently review the changed subject again.

The model received only compiler-produced context, with exact archive/view
bindings, from a separate checkout mounted read-only in an isolated runtime.
It had no tools or performer conversation. The selected model was
`qwen3.8-27b-heretic-rvn-q4_k_m`, served by local LAMU 0.6.0 (`5309d55`).
No cloud route or credentials were supplied to the runtime; its network
namespace had no outbound route.

An initial 73,728-token context produced four incomplete 64-token answers.
The strict client rejected all four; none was ingested. Source inspection
identified LAMU's conservative serialized-prompt estimate and output clamp.
A separate runtime used a 114,688-token context and q4_0 KV cache, with normal
VRAM admission. The same four inputs remained below the unchanged 60,000
estimated-token input budget, and all four subsequent answers completed.
Local compute cost was not metered; it is not reported as zero.

The host's loaded NVIDIA kernel module and installed NVML library differed.
A signature-verified cached package supplied matching NVML to these processes
only. No system driver, shared model routing or foreign process was changed.
Both private runtimes were stopped after their calls, and their snapshots
were retained before cleanup. This is runtime evidence, not operator-custody
or production-isolation qualification.

Raw provider responses, client logs, input manifests, transport failures,
ingestion results and runtime snapshots are retained under
`/mnt/4tb/artifacts/OpenWarrant/keep/owner-acts-current-integration-20261017/ow75/`.
The exact received responses and context archives also remain in this
Warrant's `verifications/` tree. No signature, resolution or assurance mark
was created.
