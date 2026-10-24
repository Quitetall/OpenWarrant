# Describe bounded retry behavior

A retry policy states the settled limits and the refusal case.

- Retry transient transport failures at most three times.
- Never retry a permission refusal.
- Keep the existing request and response contracts unchanged.
