# Design: a TOTP second factor

## Context

Sessions are cookies signed by the server.

## Decisions

### 1. TOTP, not SMS

SMS codes are intercepted; TOTP needs no network.
