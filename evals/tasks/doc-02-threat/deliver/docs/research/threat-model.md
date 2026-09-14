# Threat model

## Threats

1. An agent reaches the signing key. Control: `ssh-add -c`. Residual: the operator's discipline.
2. A pty defeats the TTY check. Control: none. Residual: accepted.
