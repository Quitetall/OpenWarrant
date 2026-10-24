# Add a transport contract and caller

A transport contract precedes its dependent caller.

- API.md defines GET /health with no request body: HTTP 200 returns {"status":"ok"}; HTTP 503 returns {"status":"unavailable"}.
- Write CLIENT.md only after API.md; map HTTP 200 to healthy and HTTP 503 to unavailable.
- A missing required caller must leave the Warrant unsatisfied.
