# Board browser observation

Candidate: 9161f97 plus reference-workbench board integration (subsequent commit).
Performed in the Codex in-app browser against a task-owned loopback server at
127.0.0.1:46007, using disposable state and an explicitly public fixture token.
No execution harness was configured. The repository was read, not changed by UI.

Observed:

- Unlock displayed the Project board with the program OpenWarrant.
- Objectives, Warrants, Stages and Open questions appeared as disclosure sections.
- Approval commands displayed 49 numbered rows, including exact `war sign` commands.
  There was no signing action control.
- Opening Stages displayed open, claimed and blocked rows with waiting dependencies.
- After stopping the owned server, Refresh board showed `Board unavailable: Failed
  to fetch` and removed previous program/stage/approval contents. The common notice
  said that no successful action was assumed.
- The temporary tab was closed and the server's terminal exit 143 was observed.

This proves these browser interactions, not every viewport, offline-file rendering,
human acceptance, or independent verification. HTTP tests separately compare board
payloads with the real CLI and exercise authentication/write refusal.
