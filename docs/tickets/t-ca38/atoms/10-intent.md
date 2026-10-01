# Ticket operations leave the corpus projections stale

Since t-67ed, the corpus projections (CORPUS_STATUS, next) include ready ticket items, so `war create`/`done` make `war check --generated` fail with drift until someone recompiles — seen when a ticket was created between a compile and a check during the 2026-09-26 evidence run (OW-WAR-0059's war-check gate failed on CORPUS_STATUS.html drift). Either ticket state stays out of committed projections (render it live), or ticket ops recompile the affected projection. Tickets are meant to be cheap; they must not break a strict drift check.
