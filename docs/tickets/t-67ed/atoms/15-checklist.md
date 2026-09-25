# Checklist

- [x] war init output and AGENTS.md: tickets first, authority as opt-in (i-685d) — done by claude, 2026-09-25: AGENTS template + legacy doc: tickets first, Warrant rules as opt-in 'When a ticket needs sign-off' (performer rules verbatim); root AGENTS.md points at them; plain init prints the start hint right under 'initialized'; --program keeps its pinned 3 lines (0e6abe45)
- [x] war next: show ready ticket items before authority acts (i-7b9b) — done by claude, 2026-09-25: war next: 'ready' list from ticket::ready_rows before judged human acts; invariant test + 67-pins-next plants (dc44f695)
- [x] Web UI and TUI: a Tickets pane (i-e7b7) — done by claude, 2026-09-25: ticket::board; TUI 't Tickets' pane (read-only, default when tickets exist); web Tickets page (default) with loopback-only claim/done via POST /api/ticket, LAN 403 act.host-only; 46-webui-tickets + 59-webui-lan plants (89a8eaef, 9ff03368)
