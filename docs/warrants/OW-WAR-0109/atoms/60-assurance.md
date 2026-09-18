---
schema: oh.war/atom/v1
warrant_uuid: 01a0b48a-8fe4-7f11-839d-3291d3dad0a2
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — visible waiting and one permitted automatic dispatch
- **scope:** configured reference queue, synthetic availability adapter and real Git/process fixture.
- **evidence:** unavailable stays queued without attempt; available creates one attempt;
  repeated polling, duplicate request and restart do not create a second writer.

### OBL-002 — queue cannot bypass current requirements
- **scope:** exact configured source/stage, configuration identity and shared admission checks.
- **evidence:** changed source/configuration, unmet dependencies, unknown writer,
  verified-start and unknown-cost cap refuse; matching ready work succeeds.

### OBL-003 — durable uncertainty and cancellation
- **scope:** queue history and dispatch crash boundaries.
- **evidence:** consumed ambiguous request is never replayed; retained matching attempt
  remains discoverable; cancelled request never dispatches; corrupt history refuses.

### OBL-004 — public interface and bounded observations
- **scope:** authenticated local API/browser and operator-configured availability probe.
- **evidence:** readable pending reasons; auth/injected command refusal; unknown,
  malformed, timeout and oversized probe output never become available; browser QA.
