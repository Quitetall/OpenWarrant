---
format: openwarrant/1.0
profile: ticket
alias: t-bf7d8a
uuid: 01a1b878-16d3-73e6-a4e4-582a0cbdc99a
priority: 2
created_at: "2026-11-08T00:04:00Z"
created_by: "claude@formal-stage-done"
summary: "Route legacy directory completion without false ticket errors or claims"
---
# Route legacy directory completion without false ticket errors or claims

## Checklist

## CHK-test-1 · check — test-1
run: cargo test -p openwarrant-cli legacy_completion --lib
on: done
evaluates t-bf7d8a

## CHK-test-2 · check — test-2
run: cargo test --locked --offline -p openwarrant-cli --test all legacy_completion
on: done
evaluates t-bf7d8a

## CHK-test-3 · check — test-3
run: cargo test --locked --offline -p openwarrant-cli --test all json_envelope::the_corpus_projection_is_the_result_payload_verbatim -- --exact
on: done
evaluates t-bf7d8a
