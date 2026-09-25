# shellcheck shell=bash
# OW-WAR-0138 — presence read from the signature, and a signature by another
# key refused for the actor.
#
# A scratch program and three keys generated here: a software key shaped as a
# FIDO security key (sk-ssh-ed25519), an ordinary ed25519 key, and an agent's
# key. No authenticator is touched: the security key's signing rule
# (PROTOCOL.u2f — sha256(application) ‖ flags ‖ counter ‖ sha256(message)) is
# applied in software by a throwaway agent started here, which holds exactly
# these three keys and signs with the flags this plant tells it to. What this
# observes is `war` reading and enforcing the flags a signature carries; it
# says nothing about any physical key's behaviour.
#
# The store half of OBL-001 and all of OBL-003 wait on U-001 and are not here.

echo "== human authentication: presence and key binding (OW-WAR-0138) =="
for SK_NEED in python3 openssl ssh-keygen; do
    command -v "$SK_NEED" >/dev/null 2>&1 || { printf 'PLANT SETUP FAILED: %s is not installed; presence is UNKNOWN here, not passed\n' "$SK_NEED" >&2; exit 9; }
done
PLANT_ROOT=$(scratch_corpus SK)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
SK_TMP=$(mktemp -d)
SK_RESP="$PLANT_ROOT/docs/authority/responses"

# The throwaway agent. Stdlib Python for the protocol, openssl for ed25519.
cat > "$SK_TMP/agent.py" <<'PY'
import os, socket, struct, subprocess, sys, tempfile, hashlib, base64

def s(b): return struct.pack(">I", len(b)) + b
SK = b"sk-ssh-ed25519@openssh.com"
ED = b"ssh-ed25519"
APP = b"ssh:"

def raw_pub(pem):
    der = subprocess.run(["openssl", "pkey", "-in", pem, "-pubout", "-outform", "DER"],
                         capture_output=True, check=True).stdout
    return der[-32:]

def blob(pem, kind):
    pk = raw_pub(pem)
    return s(SK) + s(pk) + s(APP) if kind == "sk" else s(ED) + s(pk)

def ed_sign(pem, data):
    with tempfile.NamedTemporaryFile(delete=False) as f:
        f.write(data); path = f.name
    try:
        return subprocess.run(["openssl", "pkeyutl", "-sign", "-rawin", "-inkey", pem, "-in", path],
                              capture_output=True, check=True).stdout
    finally:
        os.unlink(path)

def sign(pem, kind, data, mode):
    if kind != "sk":
        return s(ED) + s(ed_sign(pem, data))
    flags = 0x01 if mode in ("present", "flip") else 0x00
    counter = struct.pack(">I", 7)
    tbs = hashlib.sha256(APP).digest() + bytes([flags]) + counter + hashlib.sha256(data).digest()
    sig = ed_sign(pem, tbs)
    if mode == "flip":
        flags ^= 0x01  # the byte after signing: covered, so this must not verify
    return s(SK) + s(sig) + bytes([flags]) + counter

def serve(sock_path, keys, mode_file):
    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(sock_path); srv.listen(4)
    while True:
        c, _ = srv.accept()
        with c:
            while True:
                head = c.recv(4, socket.MSG_WAITALL)
                if len(head) < 4: break
                (n,) = struct.unpack(">I", head)
                msg = c.recv(n, socket.MSG_WAITALL)
                t, body = msg[0], msg[1:]
                if t == 11:
                    out = bytes([12]) + struct.pack(">I", len(keys))
                    for b, pem, kind, name in keys:
                        out += s(b) + s(name.encode())
                elif t == 13:
                    (kl,) = struct.unpack(">I", body[:4]); kb = body[4:4 + kl]
                    rest = body[4 + kl:]; (dl,) = struct.unpack(">I", rest[:4]); data = rest[4:4 + dl]
                    hit = [k for k in keys if k[0] == kb]
                    if not hit:
                        out = bytes([5])
                    else:
                        mode = open(mode_file).read().strip()
                        out = bytes([14]) + s(sign(hit[0][1], hit[0][2], data, mode))
                else:
                    out = bytes([5])
                c.sendall(s(out))

if sys.argv[1] == "pub":
    print((SK if sys.argv[3] == "sk" else ED).decode() + " " + base64.b64encode(blob(sys.argv[2], sys.argv[3])).decode())
else:
    sock_path, mode_file = sys.argv[2], sys.argv[3]
    keys = []
    for spec in sys.argv[4:]:
        pem, kind, name = spec.split(":")
        keys.append((blob(pem, kind), pem, kind, name))
    serve(sock_path, keys, mode_file)
PY
for SK_K in human plain bot; do
    openssl genpkey -algorithm ed25519 -out "$SK_TMP/$SK_K.pem" 2>/dev/null \
        || { printf 'PLANT SETUP FAILED: openssl cannot make an ed25519 key\n' >&2; exit 9; }
done
SK_HUMAN_PUB=$(python3 "$SK_TMP/agent.py" pub "$SK_TMP/human.pem" sk)
SK_PLAIN_PUB=$(python3 "$SK_TMP/agent.py" pub "$SK_TMP/plain.pem" ed)
SK_BOT_PUB=$(python3 "$SK_TMP/agent.py" pub "$SK_TMP/bot.pem" ed)
printf '%s bot\n' "$SK_BOT_PUB" > "$SK_TMP/bot.pub"
{
    printf 'human namespaces="oh.war/response,oh.war/dsse" %s\n' "$SK_HUMAN_PUB"
    printf 'plain namespaces="oh.war/response,oh.war/dsse" %s\n' "$SK_PLAIN_PUB"
    printf 'bot namespaces="oh.war/response,oh.war/dsse" %s\n' "$SK_BOT_PUB"
} > "$PLANT_ROOT/docs/authority/allowed_signers"
cat > "$PLANT_ROOT/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Key Holder"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/58-authn.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Signs with the security-key-shaped key. Exists only while these plants run."
ssh_principal = "human"

[[assignment]]
actor = "Plain Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/58-authn.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Signs with an ordinary ed25519 key. Exists only while these plants run."
ssh_principal = "plain"

[[assignment]]
actor = "claude"
actor_kind = "agent"
roles = ["performer"]
assigned_by = "conformance/plants.d/58-authn.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "The agent's key, listed so a signature by it has a name."
ssh_principal = "bot"
ROLES
for SK_N in 2 3 4 5; do
    "$WAR" --root "$PLANT_ROOT" new "Presence plant Warrant $SK_N" >/dev/null 2>&1
done
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "register three keys, five Warrants" >/dev/null 2>&1

SK_OLD_SOCK=${SSH_AUTH_SOCK:-}
SK_OLD_PID=${SSH_AGENT_PID:-}
unset SSH_AGENT_PID
export SSH_AUTH_SOCK="$SK_TMP/agent.sock"
printf 'present\n' > "$SK_TMP/mode"
python3 "$SK_TMP/agent.py" serve "$SSH_AUTH_SOCK" "$SK_TMP/mode" \
    "$SK_TMP/human.pem:sk:human" "$SK_TMP/plain.pem:ed:plain" "$SK_TMP/bot.pem:ed:bot" &
SK_AGENT=$!
for _ in $(seq 50); do [[ -S "$SSH_AUTH_SOCK" ]] && break; sleep 0.1; done
sk_teardown() {
    kill "$SK_AGENT" 2>/dev/null; wait "$SK_AGENT" 2>/dev/null
    if [[ -n "$SK_OLD_SOCK" ]]; then export SSH_AUTH_SOCK="$SK_OLD_SOCK"; else unset SSH_AUTH_SOCK; fi
    if [[ -n "$SK_OLD_PID" ]]; then export SSH_AGENT_PID="$SK_OLD_PID"; else unset SSH_AGENT_PID; fi
    command rm -rf "$SK_TMP"
}
# The agent must hold exactly the three keys made here, and nothing else.
SK_WANT=$(printf '%s\n' "$SK_HUMAN_PUB" "$SK_PLAIN_PUB" "$SK_BOT_PUB" | sort)
SK_HELD=$(ssh-add -L 2>/dev/null | cut -d' ' -f1,2 | sort)
if [[ "$SK_HELD" != "$SK_WANT" ]]; then
    printf 'PLANT SETUP FAILED: the throwaway agent does not hold exactly the plant keys\n' >&2
    sk_teardown; exit 9
fi

sk_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sk_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
sk_sign() { # alias, actor, [extra args] → output; status in SK_STATUS
    local a=$1 who=$2; shift 2
    SK_OUT=$("$WAR" --root "$PLANT_ROOT" sign "$a" --ssh-sign --as "$who" "$@" </dev/null 2>&1); SK_STATUS=$?
}
sk_nothing() { # alias → true when no record and no response (or draft, or .sig) exist for it
    [[ ! -e "$PLANT_ROOT/docs/warrants/$1/authorization.toml" ]] \
        && [[ -z "$(ls "$SK_RESP/$1".* 2>/dev/null)" ]]
}
sk_predicate() { # output → the predicate's presence, from the attestation it names
    local att
    att=$(grep -o 'attestation written to [^ ]*' <<<"$1" | head -1 | awk '{print $4}')
    [[ -n "$att" ]] || { echo "no-attestation"; return; }
    python3 - "$PLANT_ROOT/$att" <<'PY'
import base64, json, sys
env = json.load(open(sys.argv[1]))
stmt = json.loads(base64.b64decode(env["payload"]))
print(stmt["predicate"].get("presence", "absent"))
PY
}
sk_policy() { # on|off
    if [[ "$1" == on ]]; then
        python3 - "$PLANT_ROOT/openwarrant.toml" <<'PY'
import re, sys
p = sys.argv[1]; t = open(p).read()
t = re.sub(r"(?m)^require_user_presence = .*\n", "", t)
t = re.sub(r"(?m)^\[policy\]$", "[policy]\nrequire_user_presence = true", t, count=1) if "[policy]" in t else t + "\n[policy]\nrequire_user_presence = true\n"
open(p, "w").write(t)
PY
        grep -q '^require_user_presence = true' "$PLANT_ROOT/openwarrant.toml" \
            || { printf 'PLANT SETUP FAILED: could not set require_user_presence\n' >&2; sk_teardown; exit 9; }
    else
        git -C "$PLANT_ROOT" checkout -q -- openwarrant.toml
    fi
}

# OBL-002, policy off: both keys sign, and the record says which.
sk_sign SK-WAR-0001 "Plain Signer"
if [[ $SK_STATUS -eq 0 ]] && grep -q 'sign.presence .*ssh-ed25519 key: presence unverified' <<<"$SK_OUT" \
    && [[ "$(sk_predicate "$SK_OUT")" == unverified ]]; then
    sk_ok "an ordinary key signs, unverified" "sign.presence unverified; attestation predicate says so"
else
    sk_fail "an ordinary key signs, unverified" "exit $SK_STATUS, predicate $(sk_predicate "$SK_OUT"): $(grep -E '^(ERROR|PASS sign.presence)' <<<"$SK_OUT" | head -2 | tr '\n' '|')"
fi
sk_sign SK-WAR-0002 "Key Holder"
if [[ $SK_STATUS -eq 0 ]] && grep -q 'sign.presence .*sk-ssh-ed25519@openssh.com key, flags 0x01, counter 7: presence verified' <<<"$SK_OUT" \
    && [[ "$(sk_predicate "$SK_OUT")" == verified ]]; then
    sk_ok "a touched security key, verified" "flags 0x01 read from the signature; predicate verified"
else
    sk_fail "a touched security key, verified" "exit $SK_STATUS, predicate $(sk_predicate "$SK_OUT"): $(grep -E '^(ERROR|PASS sign.presence)' <<<"$SK_OUT" | head -2 | tr '\n' '|')"
fi

# OBL-002, policy on: nothing that does not show presence is recorded.
sk_policy on
sk_sign SK-WAR-0003 "Plain Signer"
if [[ $SK_STATUS -ne 0 ]] && grep -q 'ERROR sign.presence-required' <<<"$SK_OUT" && sk_nothing SK-WAR-0003; then
    sk_ok "an ordinary key, refused by policy" "sign.presence-required; no record, no response"
else
    sk_fail "an ordinary key, refused by policy" "exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT" | head -2 | tr '\n' '|') $(ls "$SK_RESP" | grep SK-WAR-0003 | tr '\n' ' ')"
fi
printf 'absent\n' > "$SK_TMP/mode"
sk_sign SK-WAR-0003 "Key Holder"
if [[ $SK_STATUS -ne 0 ]] && grep -q 'ERROR sign.presence-required .*flags 0x00' <<<"$SK_OUT" && sk_nothing SK-WAR-0003; then
    sk_ok "a security key with no touch, refused" "flags 0x00 → sign.presence-required; nothing recorded"
else
    sk_fail "a security key with no touch, refused" "exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT" | head -2 | tr '\n' '|')"
fi
printf 'flip\n' > "$SK_TMP/mode"
sk_sign SK-WAR-0003 "Key Holder"
if [[ $SK_STATUS -ne 0 ]] && grep -q 'ERROR sign.ssh-refused .*does not verify' <<<"$SK_OUT" \
    && ! grep -q 'sign.presence ' <<<"$SK_OUT" && sk_nothing SK-WAR-0003; then
    sk_ok "a flipped flag byte fails verification" "sign.ssh-refused before any presence is read"
else
    sk_fail "a flipped flag byte fails verification" "exit $SK_STATUS: $(grep -E '^(ERROR|PASS sign.presence)' <<<"$SK_OUT" | head -2 | tr '\n' '|')"
fi

# OBL-001 (the part that does not wait on a store): a signature by a key
# bound to someone else is refused for the actor named by --as. The shim
# stands in for whatever makes such a signature — it hands `ssh-keygen -Y
# sign` the agent's own key in place of the one war asked for.
printf 'present\n' > "$SK_TMP/mode"
mkdir -p "$SK_TMP/shim"
SK_REAL_KEYGEN=$(command -v ssh-keygen)
cat > "$SK_TMP/shim/ssh-keygen" <<SHIM
#!/usr/bin/env bash
args=("\$@")
if [[ "\${1:-}" == "-Y" && "\${2:-}" == "sign" ]]; then
    for i in "\${!args[@]}"; do [[ "\${args[\$i]}" == "-f" ]] && args[\$((i + 1))]="$SK_TMP/bot.pub"; done
fi
exec "$SK_REAL_KEYGEN" "\${args[@]}"
SHIM
chmod +x "$SK_TMP/shim/ssh-keygen"
SK_OUT=$(PATH="$SK_TMP/shim:$PATH" "$WAR" --root "$PLANT_ROOT" sign SK-WAR-0003 --ssh-sign --as "Key Holder" </dev/null 2>&1); SK_STATUS=$?
if [[ $SK_STATUS -ne 0 ]] && grep -q 'ERROR sign.actor-key-mismatch .*binds to \["bot"\]' <<<"$SK_OUT" && sk_nothing SK-WAR-0003; then
    sk_ok "another actor's key, refused" "sign.actor-key-mismatch names the key's principal; nothing recorded"
else
    sk_fail "another actor's key, refused" "exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT" | head -2 | tr '\n' '|')"
fi

# The positive under the policy: the bound human's touched key is recorded,
# and the record verifies on the read path.
sk_sign SK-WAR-0003 "Key Holder"
SK_CHECK=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
SK_VERIFY=$("$WAR" --root "$PLANT_ROOT" sign SK-WAR-0003 --verify 2>&1)
if [[ $SK_STATUS -eq 0 ]] && [[ -f "$PLANT_ROOT/docs/warrants/SK-WAR-0003/authorization.toml" ]] \
    && grep -q 'PASS authority.signed .*SK-WAR-0003' <<<"$SK_CHECK" \
    && grep -q 'PASS sign.presence .*presence verified' <<<"$SK_VERIFY"; then
    sk_ok "the bound human signs under policy" "recorded; authority.signed; --verify reads presence verified"
else
    sk_fail "the bound human signs under policy" "exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT$SK_CHECK" | head -2 | tr '\n' '|')"
fi
# The same signature with its flag byte flipped on disk: the read path
# refuses it outright rather than reading a presence from it.
SK_SIG="$SK_RESP/SK-WAR-0003.response.toml.sig"
command cp "$SK_SIG" "$SK_TMP/kept.sig"
python3 - "$SK_SIG" <<'PY'
import base64, sys
p = sys.argv[1]; lines = open(p).read().split("\n")
raw = bytearray(base64.b64decode("".join(l for l in lines if l and not l.startswith("-----"))))
raw[-5] ^= 0x01
b = base64.b64encode(bytes(raw)).decode()
open(p, "w").write("-----BEGIN SSH SIGNATURE-----\n" + "\n".join(b[i:i + 70] for i in range(0, len(b), 70)) + "\n-----END SSH SIGNATURE-----\n")
PY
SK_CHECK=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
SK_VERIFY=$("$WAR" --root "$PLANT_ROOT" sign SK-WAR-0003 --verify 2>&1)
command cp "$SK_TMP/kept.sig" "$SK_SIG"
if grep -q 'ERROR authority.signature-invalid .*SK-WAR-0003' <<<"$SK_CHECK" \
    && grep -q 'ERROR sign.not-verified' <<<"$SK_VERIFY" && ! grep -q 'sign.presence ' <<<"$SK_VERIFY"; then
    sk_ok "a flag flipped on disk is invalid" "authority.signature-invalid; --verify reads no presence"
else
    sk_fail "a flag flipped on disk is invalid" "$(grep -E 'SK-WAR-0003' <<<"$SK_CHECK" | grep -E '^(ERROR|PASS) authority' | head -1) $(grep -E '^(ERROR|PASS)' <<<"$SK_VERIFY" | head -2 | tr '\n' '|')"
fi

# A batch is not the way around the policy (AM-003).
sk_batch() { # actor → output; status in SK_STATUS
    SK_OUT=$("$WAR" --root "$PLANT_ROOT" sign --batch SK-WAR-0004,SK-WAR-0005 --ssh-sign --as "$1" </dev/null 2>&1); SK_STATUS=$?
}
sk_batch "Plain Signer"
if [[ $SK_STATUS -ne 0 ]] && grep -q 'ERROR sign.presence-required' <<<"$SK_OUT" && sk_nothing SK-WAR-0004 && sk_nothing SK-WAR-0005 \
    && [[ -z "$(ls "$PLANT_ROOT/docs/authority/batches" 2>/dev/null)" ]]; then
    sk_ok "a batch by an ordinary key, refused" "sign.presence-required; no batch, no record"
else
    sk_fail "a batch by an ordinary key, refused" "exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT" | head -2 | tr '\n' '|')"
fi
sk_batch "Key Holder"
if [[ $SK_STATUS -eq 0 ]] && grep -q 'batch.recorded' <<<"$SK_OUT" && grep -q 'PASS sign.presence .*presence verified' <<<"$SK_OUT" \
    && [[ -f "$PLANT_ROOT/docs/warrants/SK-WAR-0004/authorization.toml" && -f "$PLANT_ROOT/docs/warrants/SK-WAR-0005/authorization.toml" ]]; then
    sk_ok "a batch by a touched key, recorded" "batch.recorded, presence verified"
else
    sk_fail "a batch by a touched key, recorded" "exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT" | head -2 | tr '\n' '|')"
fi
sk_policy off

sk_teardown
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
