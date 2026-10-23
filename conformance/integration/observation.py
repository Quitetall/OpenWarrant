# SPDX-License-Identifier: Apache-2.0
"""Shared output handling for integration observations, never authority acts."""
import json
import os
import tempfile


def require_new_destination(path):
    if os.path.lexists(path):
        raise FileExistsError(f"Observation destination exists: {path}")


def publish_observation(path, receipt):
    raw = (json.dumps(receipt, indent=2) + "\n").encode("utf-8")
    path.parent.mkdir(parents=True, exist_ok=True)
    # Publish a complete file with an exclusive link. A destination created
    # after preflight still wins; it is never replaced or followed.
    with tempfile.NamedTemporaryFile(dir=path.parent, prefix=".ow-observation-") as output:
        output.write(raw)
        output.flush()
        os.fsync(output.fileno())
        os.link(output.name, path)
