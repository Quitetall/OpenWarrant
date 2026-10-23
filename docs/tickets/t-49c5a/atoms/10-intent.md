# Keep private Git index copies faithful to source timestamps

Controlled exact API probe: native Git reports first tree unchanged and second tree changed, while the private-index observer reports the opposite. fs::copy changes index mtime and invalidates Git racy-stat assumptions. Evidence: /mnt/4tb/artifacts/OpenWarrant/keep/gate-stream-capture-20261021/racy-index/results.json. Preserve real index bytes, read-only behavior, strict unknown on observation errors, and source path coverage. This is distinct from the stream-capture repair.
