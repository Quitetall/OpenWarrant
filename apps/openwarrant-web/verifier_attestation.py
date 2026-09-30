# SPDX-License-Identifier: Apache-2.0
"""Authenticate machine-issued protection observations; never human acceptance."""
import re
import subprocess
import tempfile
from pathlib import Path

from verification import require, bounded, identity
from verifier_policy import PROTECTIONS

NAMESPACE = "openwarrant-harness-protection"


def authenticate(payload, signature, *, public_key, principal, basis_sha256, nonce, now, decode):
    """Verify exact received bytes using an operator-pinned harness public key.

    The harness must protect its signing key and actually enforce the observations.
    A valid signature proves origin/binding, not that a dishonest issuer is correct.
    Caller must reserve the nonce and prevent a second dispatch with that claim.
    """
    require(isinstance(payload, bytes) and 0 < len(payload) <= 16384
            and isinstance(signature, bytes) and 0 < len(signature) <= 16384,
            "Bounded signed protection evidence required")
    require(isinstance(principal, str) and re.fullmatch(r"[A-Za-z0-9_.-]{1,128}", principal),
            "Exact configured harness principal required")
    require(isinstance(public_key, str) and re.fullmatch(r"ssh-ed25519 [A-Za-z0-9+/]+={0,2}", public_key),
            "Pinned Ed25519 harness public key required")
    require(isinstance(basis_sha256, str) and re.fullmatch(r"[0-9a-f]{64}", basis_sha256)
            and identity(nonce) and type(now) is int, "Exact admission basis, claim nonce and current time required")
    with tempfile.TemporaryDirectory(prefix="ow-verifier-attestation-") as tmp:
        root = Path(tmp)
        allowed = root / "allowed-signers";signed = root / "signature"
        allowed.write_text(principal + " " + public_key + "\n")
        signed.write_bytes(signature)
        completed = subprocess.run(["ssh-keygen", "-Y", "verify", "-f", str(allowed),
            "-I", principal, "-n", NAMESPACE, "-s", str(signed)], input=payload,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5, check=False)
        require(completed.returncode == 0, "Harness protection signature refused")
    r = decode(payload)
    require(isinstance(r, dict) and set(r) == {
        "schema", "basis_sha256", "nonce", "issued_at_unix", "expires_at_unix", "evidence_ref", "protections"
    } and r["schema"] == "oh.war/harness-protection/v1", "Invalid signed protection record")
    require(r["basis_sha256"] == basis_sha256 and r["nonce"] == nonce, "Protection evidence belongs to another claim")
    require(type(r["issued_at_unix"]) is int and type(r["expires_at_unix"]) is int
            and r["issued_at_unix"] <= now < r["expires_at_unix"] <= r["issued_at_unix"] + 300,
            "Protection evidence expired, future-dated or overlong")
    require(bounded(r["evidence_ref"], 2000) and isinstance(r["protections"], dict)
            and set(r["protections"]) == PROTECTIONS
            and all(v in ("pass", "fail", "unknown") for v in r["protections"].values()),
            "Exact protection observations required")
    return {k: r[k] for k in ("basis_sha256", "evidence_ref", "protections")}
