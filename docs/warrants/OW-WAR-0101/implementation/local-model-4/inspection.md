# Loading refusal

The adapter contacted the explicitly selected local llama-server before its load
finished. The server returned HTTP 503 with Loading model; the adapter exited 1
without proposal stdout. No inference result or task-fidelity result is claimed.
The same process subsequently reported loaded; its one bounded retry is retained
separately under local-model-4-ready/. Neither attempt applied documents.
