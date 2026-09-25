# Ticket bookkeeping must not make evidence stale

war claim/done/note write docs/tickets/*/journal.jsonl and the checklist; the tree binding (gate_cmd.rs source::Exclusions) does not skip them, so working tickets after evidence is recorded stales every receipt from a gate with no declared inputs. Found by t-22fd.
