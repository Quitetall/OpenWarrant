#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Public CLI fixture: synthetic test authority, never owner activation."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

repo, war = map(Path, sys.argv[1:3])
with tempfile.TemporaryDirectory(prefix="ow-role-consumers-") as temporary:
    root = Path(temporary)
    project = root / "project"
    shutil.copytree(repo / "conformance/fixtures/inbox/repository", project)
    store = root / "store"
    store.mkdir()
    revision = {"schema":"oh.war/authority-revision/2", "repository":"synthetic-role-control", "sequence":0,
        "principals":{"owner":{"public_key":"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "roles":["authority-admin","authorizer","resolver"],"actor":"Synthetic Store Human","kind":"human"}},
        "policy":{"allow_automated_resolution":False,"require_user_presence":True,"verifier_argv":[]}}
    snapshot = {"schema":"oh.war/authority-store/1","agent_uid":None,"unprotected_test_store":True,
        "genesis":revision,"legacy":{},"transitions":[]}
    state = store / "state.json"
    state.write_text(json.dumps(snapshot,sort_keys=True,separators=(",",":")))
    state.chmod(0o400); store.chmod(0o500)
    config = project / "openwarrant.toml"
    config.write_text(config.read_text()+f'\n[authority]\nstore = {json.dumps(str(store))}\nunprotected_test_store = true\n')
    def request():
        result = subprocess.run([str(war),"--root",str(project),"authorize","IX-WAR-0003","--json"],capture_output=True,timeout=20)
        assert result.returncode == 0, result.stderr.decode()+result.stdout.decode()
        answer = json.loads(result.stdout)
        assert answer["result"]["eligible_authorizers"] == ["Synthetic Store Human"], answer["result"]["eligible_authorizers"]
    try:
        request()  # Legacy signer names must never replace the protected grant.
        roles = project / "docs/authority/roles.toml"
        roles.write_text("[[assignment]]\nactor = 1\n")
        request()  # A malformed irrelevant legacy file cannot hide this grant.
        store.chmod(0o700)
        state.unlink()
        store.chmod(0o500)
        refused = subprocess.run([str(war),"--root",str(project),"authorize","IX-WAR-0003","--json"],capture_output=True,timeout=20)
        assert refused.returncode != 0, "missing configured store must never fall back"
    finally:
        store.chmod(0o700)
print("protected grant visible; malformed legacy ignored; unavailable store refused")
