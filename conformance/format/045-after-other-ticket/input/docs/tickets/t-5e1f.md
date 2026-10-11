---
format: openwarrant/1.0
profile: ticket
alias: t-5e1f
uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd50
summary: Send the password reset email.
priority: 2
labels:
  - auth
  - email
---
# Reset email

## Checklist
- [ ] Email template (i-4d11)
- [ ] Docs (i-77be, after i-4d11)
- [ ] Announce (i-0a0a, after t-0d61d/i-9c01, after t-0d61d)
- [x] Reset endpoint (i-9c01) — done by claude, 2026-09-25: POST /reset
