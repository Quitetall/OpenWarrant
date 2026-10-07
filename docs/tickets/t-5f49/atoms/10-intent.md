# war ui answers 431 instead of 413 for an oversized body, under load

63-webui-refusals.sh 'oversized bodies under load' (added by t-26ca) saw 49 × 413 and 1 × 431 on OW-WAR-0114's evidence run (three batteries at once, 2026-09-26). The request carried normal headers and an oversized body; 431 (headers too large) is the wrong refusal. Likely the header reader, under a slow/partial read, sees body bytes before the blank line or misjudges the header section's length. Both answers refuse; the misclassification tells a client the wrong thing.
