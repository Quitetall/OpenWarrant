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
# The store half of OBL-001 and all of OBL-003 are at the end, CONTINGENT on
# the owner answering OW-WAR-0138 U-001 option A. What they model and what
# they cannot observe is said there.

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
    chmod -R u+rwx "$SK_TMP" 2>/dev/null
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

# ---------------------------------------------------------------------------
# The store half: OBL-001's store plants and OBL-003. CONTINGENT on the owner
# answering OW-WAR-0138 U-001 option A — opt-in per repository; everywhere
# else `war check` warns `authority.unprotected`; a configured store governs,
# with no fallback.
#
# The store's separate OS account cannot be created in a test. It is modelled
# as the work order allows: a `--unprotected-test-store` in this plant's
# scratch directory, owned by the account running the plant, and sealed
# read-only (directory 0500, state.json 0400) so the reading process cannot
# write it — `war` refuses the test store while it can. The "operator" below
# is this same account unsealing it for one activation. What is observed is
# `war` reading, obeying and failing closed on a store; no OS boundary, no
# second account and no deployed store's protection is observed (R-003).
# ---------------------------------------------------------------------------
ST_DIR="$SK_TMP/store"
mkdir -m 700 "$ST_DIR"
printf '%s plain\n' "$SK_PLAIN_PUB" > "$SK_TMP/plain.pub"
st_war() { "$WAR" --root "$PLANT_ROOT" "$@" </dev/null 2>&1; }
st_seal() { chmod 0400 "$ST_DIR/state.json"; chmod 0500 "$ST_DIR"; }
st_unseal() { chmod 0700 "$ST_DIR"; chmod 0600 "$ST_DIR/state.json"; }
st_setup_failed() { printf 'PLANT SETUP FAILED: %s\n' "$1" >&2; sk_teardown; exit 9; }
st_config() { # on [protected] | off — the [authority] table, never committed
    git -C "$PLANT_ROOT" checkout -q -- openwarrant.toml
    if [[ "$1" == on ]]; then
        printf '\n[authority]\nstore = "%s"\n' "$ST_DIR" >> "$PLANT_ROOT/openwarrant.toml"
        [[ "${2:-}" == protected ]] || printf 'unprotected_test_store = true\n' >> "$PLANT_ROOT/openwarrant.toml"
    fi
    return 0
}
st_toml() { # table, "key = value" → set under that table of openwarrant.toml
    python3 - "$PLANT_ROOT/openwarrant.toml" "$1" "$2" <<'EOP'
import re, sys
p, table, line = sys.argv[1], sys.argv[2], sys.argv[3]
t = open(p).read()
key = line.split("=")[0].strip()
if re.search(r"(?m)^\[" + re.escape(table) + r"\]$", t):
    t = re.sub(r"(?m)^" + re.escape(key) + r" = .*\n", "", t)
    t = re.sub(r"(?m)^\[" + re.escape(table) + r"\]$", "[" + table + "]\n" + line, t, count=1)
else:
    t += "\n[" + table + "]\n" + line + "\n"
open(p, "w").write(t)
EOP
}
st_policy() { # file, require_user_presence → a policy table mirroring this corpus's own values
    python3 - "$PLANT_ROOT/openwarrant.toml" "$1" "$2" <<'EOP'
import json, sys, tomllib
c = tomllib.load(open(sys.argv[1], "rb"))
p = {"allow_automated_resolution": c.get("policy", {}).get("allow_automated_resolution", False),
     "require_user_presence": sys.argv[3] == "true",
     "verifier_argv": c.get("verify", {}).get("verifier_argv", [])}
if "independence" in c:
    p["independence"] = c["independence"]
json.dump(p, open(sys.argv[2], "w"))
EOP
}
st_transition() { # name, draft args… → a transition signed by the store's admin and activated
    local n=$1; shift
    st_unseal
    "$WAR" authority status --store "$ST_DIR" --emit "$SK_TMP/$n.cur.json" --unprotected-test-store >/dev/null 2>&1 \
        && "$WAR" authority draft --current "$SK_TMP/$n.cur.json" --principal plain "$@" --emit "$SK_TMP/$n.next.json" >/dev/null 2>&1 \
        && "$WAR" authority propose --current "$SK_TMP/$n.cur.json" --next "$SK_TMP/$n.next.json" --emit "$SK_TMP/$n.change.json" >/dev/null 2>&1 \
        && "$WAR" authority approve --store "$ST_DIR" --proposal "$SK_TMP/$n.change.json" --principal plain \
            --key "$SK_TMP/plain.pub" --activate --unprotected-test-store >/dev/null 2>&1 \
        || st_setup_failed "store transition $n did not activate"
    st_seal
}
st_head() { "$WAR" authority status --store "$ST_DIR" --unprotected-test-store --json 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["head"])'; }

for SK_N in 6 7 8 9 10 11; do
    "$WAR" --root "$PLANT_ROOT" new "Store plant Warrant $SK_N" >/dev/null 2>&1
done
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1

# The performer's move, in the working tree: its own key under a new
# principal, and a human-kind entry naming it. Neither is in any store.
printf 'mallory namespaces="oh.war/response,oh.war/dsse" %s\n' "$SK_BOT_PUB" >> "$PLANT_ROOT/docs/authority/allowed_signers"
cat >> "$PLANT_ROOT/docs/authority/roles.toml" <<'ROLES'

[[assignment]]
actor = "Agent As Human"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "the performer, editing roles.toml"
effective_time = "2026-01-01T00:00:00Z"
note = "Added by the agent. Its key is the agent's own."
ssh_principal = "mallory"
ROLES
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "six Warrants; the performer's key registered as a human" >/dev/null 2>&1

# Without a store: the forgery works — that is THREAT_MODEL row 6 today, and
# what the store half exists to refuse. An authorization signed through
# `war sign`, and a resolution forged on disk and signed with the same key.
sk_sign SK-WAR-0006 "Agent As Human"
SK_FORGED=$SK_STATUS
cat > "$PLANT_ROOT/docs/warrants/SK-WAR-0007/resolution.toml" <<'RES'
schema = "oh.war/resolution/v1"
warrant = "SK-WAR-0007"

[resolution]
id = "01a06513-0395-7a21-81a3-9bfa6b5a0e49"
common_outcome = "satisfied"
profile_outcome = "delivered"
contract_revision = 1
contract_digest = "5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e"
assurance_case_snapshot_digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"
artifact_manifest_digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"
gate_run_refs = []
judgment_refs = []
residual_risk_refs = []
resolved_by_ref = "person://Agent As Human"
acting_role_ref = "role-assignment://Agent As Human/resolver"
meaning = "Forged by the performer in a plant."
effective_at = "2026-01-01T00:00:00Z"
recorded_at = "2026-01-01T00:00:00Z"
standing = "valid"
RES
cat > "$SK_RESP/SK-WAR-0007.resolution.response.toml" <<'RES'
schema = "oh.war/resolution-response/v1"
warrant = "SK-WAR-0007"
contract_digest = "5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e"
resolved_by = "Agent As Human"
acting_role = "resolver"
common_outcome = "satisfied"
profile_outcome = "delivered"
meaning = "Forged by the performer in a plant."
effective_time = "2026-01-01T00:00:00Z"
RES
ssh-keygen -Y sign -f "$SK_TMP/bot.pub" -n oh.war/response "$SK_RESP/SK-WAR-0007.resolution.response.toml" >/dev/null 2>&1 \
    || st_setup_failed "the plant agent would not sign the forged resolution"
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "forged acts" >/dev/null 2>&1
SK_CHECK=$(st_war check)
if [[ $SK_FORGED -eq 0 ]] && [[ $(grep -c '^WARN authority.unprotected' <<<"$SK_CHECK") -eq 1 ]] \
    && grep -q 'PASS authority.signed .*SK-WAR-0006: signed by mallory' <<<"$SK_CHECK" \
    && grep -q 'PASS authority.signed .*SK-WAR-0007 resolution: signed by mallory' <<<"$SK_CHECK" \
    && grep -q 'PASS authority.signed .*SK-WAR-0003' <<<"$SK_CHECK"; then
    sk_ok "no store: works, warns unprotected" "one authority.unprotected; the register's forged human verifies (row 6 today)"
else
    sk_fail "no store: works, warns unprotected" "sign exit $SK_FORGED; $(grep -cE '^WARN authority.unprotected' <<<"$SK_CHECK") warning(s); $(grep -E 'SK-WAR-000[67]' <<<"$SK_CHECK" | grep -E '^(ERROR|PASS) authority' | head -2 | tr '\n' '|')"
fi

# A v1 store (0096's shape: keys and roles, no actor kind, no policy).
"$WAR" authority draft --repository sk-plant --principal plain --public-key "$SK_TMP/plain.pub" \
    --role authority-admin --emit "$SK_TMP/genesis.json" >/dev/null 2>&1 || st_setup_failed "draft genesis"
ST_GENESIS=$(python3 - "$SK_TMP/genesis.json" <<'EOP'
import hashlib, sys
print("sha256:" + hashlib.sha256(b"openwarrant-authority-revision-v1\0" + open(sys.argv[1], "rb").read()).hexdigest())
EOP
)
"$WAR" authority bootstrap --store "$ST_DIR" --revision "$SK_TMP/genesis.json" --expected-digest "$ST_GENESIS" \
    --unprotected-test-store >/dev/null 2>&1 || st_setup_failed "bootstrap the test store"
st_seal
st_config on
SK_STATUS_OUT=$("$WAR" authority status --store "$ST_DIR" --unprotected-test-store --json 2>&1)
SK_CHECK=$(st_war check)
sk_sign SK-WAR-0008 "Plain Signer"
if grep -q '"schema": "oh.war/authority-revision/1"' <<<"$SK_STATUS_OUT" \
    && grep -q '^ERROR authority.store-v1' <<<"$SK_CHECK" \
    && [[ $SK_STATUS -ne 0 ]] && grep -q 'ERROR authority.legacy-fallback .*v1 revision' <<<"$SK_OUT" && sk_nothing SK-WAR-0008; then
    sk_ok "a v1 store loads and grants nobody" "status reads v1; authority.store-v1; a sign refused, nothing recorded"
else
    sk_fail "a v1 store loads and grants nobody" "$(grep -cE 'authority-revision/1' <<<"$SK_STATUS_OUT") v1; $(grep -E '^ERROR authority.store' <<<"$SK_CHECK" | head -1) | sign exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT" | head -1)"
fi

# v1 → v2: a signed transition like any other. Unsigned, it is refused; its
# signature moved onto a policy the admin did not sign, it is refused.
st_policy "$SK_TMP/policy.json" false
st_unseal
"$WAR" authority status --store "$ST_DIR" --emit "$SK_TMP/up.cur.json" --unprotected-test-store >/dev/null 2>&1
"$WAR" authority draft --current "$SK_TMP/up.cur.json" --v2 --policy "$SK_TMP/policy.json" --principal plain \
    --actor "Plain Signer" --kind human --role authority-admin --role authorizer --role resolver \
    --emit "$SK_TMP/up.next.json" >/dev/null 2>&1 || st_setup_failed "draft the v2 revision"
"$WAR" authority propose --current "$SK_TMP/up.cur.json" --next "$SK_TMP/up.next.json" --emit "$SK_TMP/up.change.json" >/dev/null 2>&1 \
    || st_setup_failed "propose v1 → v2"
"$WAR" authority approve --current "$SK_TMP/up.cur.json" --proposal "$SK_TMP/up.change.json" --principal plain \
    --key "$SK_TMP/plain.pub" --emit "$SK_TMP/up.sig" >/dev/null 2>&1 || st_setup_failed "sign v1 → v2"
python3 - "$SK_TMP/up.change.json" "$SK_TMP/up.tampered.json" <<'EOP'
import json, sys
p = json.load(open(sys.argv[1]))
p["next"]["policy"]["allow_automated_resolution"] = True
open(sys.argv[2], "w").write(json.dumps(p, sort_keys=True, separators=(",", ":"), ensure_ascii=False))
EOP
ST_BEFORE=$(st_head)
SK_UNSIGNED=$("$WAR" authority activate --store "$ST_DIR" --proposal "$SK_TMP/up.change.json" --unprotected-test-store 2>&1); SK_U=$?
SK_TAMPERED=$("$WAR" authority activate --store "$ST_DIR" --proposal "$SK_TMP/up.tampered.json" \
    --signature "plain=$SK_TMP/up.sig" --unprotected-test-store 2>&1); SK_T=$?
ST_AFTER=$(st_head)
if [[ $SK_U -ne 0 ]] && grep -q 'authority-signer' <<<"$SK_UNSIGNED" \
    && [[ $SK_T -ne 0 ]] && grep -q 'authority-signature-invalid' <<<"$SK_TAMPERED" \
    && [[ -n "$ST_BEFORE" && "$ST_BEFORE" == "$ST_AFTER" ]]; then
    sk_ok "an unsigned policy change, refused" "no signature: authority-signer; a signature over other policy: invalid; head unmoved"
else
    sk_fail "an unsigned policy change, refused" "unsigned exit $SK_U, tampered exit $SK_T, head $ST_BEFORE → $ST_AFTER"
fi
"$WAR" authority activate --store "$ST_DIR" --proposal "$SK_TMP/up.change.json" \
    --signature "plain=$SK_TMP/up.sig" --unprotected-test-store >/dev/null 2>&1 || st_setup_failed "activate v1 → v2"
st_seal

# The store governs. Its human signs, and the record verifies through it.
sk_sign SK-WAR-0008 "Plain Signer"
SK_CHECK=$(st_war check)
SK_VERIFY=$(st_war sign SK-WAR-0008 --verify)
if [[ $SK_STATUS -eq 0 ]] && [[ -f "$PLANT_ROOT/docs/warrants/SK-WAR-0008/authorization.toml" ]] \
    && grep -q 'PASS authority.signed .*SK-WAR-0008: signed by plain .*bound by the store .*UNPROTECTED TEST STORE' <<<"$SK_CHECK" \
    && grep -q '^WARN authority.test-store' <<<"$SK_CHECK" && ! grep -q 'authority.unprotected' <<<"$SK_CHECK" \
    && grep -q 'PASS sign.verified' <<<"$SK_VERIFY"; then
    sk_ok "the store's human signs" "recorded; authority.signed through the store, labeled a test store"
else
    sk_fail "the store's human signs" "exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT" | head -1) $(grep -E 'SK-WAR-0008' <<<"$SK_CHECK" | grep -E '^(ERROR|PASS) authority' | head -1)"
fi

# The performer's register entries grant nothing: refused by name on the
# read path (the authorization and the resolution it forged) and on the sign
# path (a new act), with nothing recorded.
sk_sign SK-WAR-0009 "Agent As Human"
if grep -q 'ERROR authority.legacy-fallback .*SK-WAR-0006: the authorization' <<<"$SK_CHECK" \
    && grep -q 'ERROR authority.legacy-fallback .*SK-WAR-0007 resolution: the resolution' <<<"$SK_CHECK" \
    && [[ $SK_STATUS -ne 0 ]] && grep -q 'ERROR authority.legacy-fallback' <<<"$SK_OUT" && sk_nothing SK-WAR-0009; then
    sk_ok "the register's human, refused" "authority.legacy-fallback on authorize, on resolve, and at signing"
else
    sk_fail "the register's human, refused" "$(grep -E 'SK-WAR-000[67]' <<<"$SK_CHECK" | grep -E '^(ERROR|PASS) authority' | head -2 | tr '\n' '|') sign exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT" | head -1)"
fi

# --as naming the store's human, with a signature made by another key.
SK_OUT=$(PATH="$SK_TMP/shim:$PATH" "$WAR" --root "$PLANT_ROOT" sign SK-WAR-0009 --ssh-sign --as "Plain Signer" </dev/null 2>&1); SK_STATUS=$?
if [[ $SK_STATUS -ne 0 ]] && grep -q 'ERROR sign.actor-key-mismatch .*binds to no principal' <<<"$SK_OUT" && sk_nothing SK-WAR-0009; then
    sk_ok "another key under the store, refused" "sign.actor-key-mismatch; the store binds that key to nobody"
else
    sk_fail "another key under the store, refused" "exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT" | head -1)"
fi

# The same protected key verifies the DSSE trail without trusting local keys.
SK_ATTEST=$(st_war attest SK-WAR-0008 --verify)
if grep -q 'PASS attest.verified' <<<"$SK_ATTEST"; then
    sk_ok "protected attestation key lookup" "retained DSSE verifies under current store key"
else
    sk_fail "protected attestation key lookup" "$(grep -E '^(ERROR|UNKNOWN)' <<<"$SK_ATTEST" | head -2 | tr '\n' '|')"
fi

# OBL-003: openwarrant.toml edited against the store is a divergence, and
# the store's value is the one read.
st_toml policy "allow_automated_resolution = true"
SK_CHECK=$(st_war check)
st_config on
if grep -q 'ERROR policy.unprotected-divergence .*\[policy\] allow_automated_resolution is true in openwarrant.toml and false in the store' <<<"$SK_CHECK"; then
    sk_ok "allow_automated_resolution edited" "policy.unprotected-divergence names both values; the store's false governs"
else
    sk_fail "allow_automated_resolution edited" "$(grep -E '^ERROR policy' <<<"$SK_CHECK" | head -1)"
fi
python3 - "$PLANT_ROOT/openwarrant.toml" <<'EOP'
import sys
dims = ["performer_transcript_blind", "performer_rationale_blind", "separate_writable_workspace",
        "cannot_modify_subject_artifacts", "cannot_modify_gate_definition", "cannot_modify_gate_fixtures",
        "separate_context_compilation", "distinct_model_required", "distinct_human_required"]
open(sys.argv[1], "a").write("\n[independence]\n" + "".join(d + " = true\n" for d in dims))
EOP
SK_CHECK=$(st_war check)
st_config on
if grep -q 'ERROR policy.unprotected-divergence .*\[independence\] is {' <<<"$SK_CHECK" \
    && grep -q '^WARN independence.undeclared' <<<"$SK_CHECK" && ! grep -q 'independence.sufficient' <<<"$SK_CHECK"; then
    sk_ok "[independence] set true, not counted" "a divergence; war check reports the store's undeclared"
else
    sk_fail "[independence] set true, not counted" "$(grep -E '^(ERROR policy|WARN independence|PASS independence)' <<<"$SK_CHECK" | head -2 | tr '\n' '|')"
fi

# A policy key the store turns on governs a real consumer: presence.
# This case has its own disposable store. Once presence is required, the
# ordinary fixture key cannot turn it off; later revocation cases must not
# silently bypass that policy to reset their setup.
ST_WITHOUT_PRESENCE=$ST_DIR
ST_DIR="$SK_TMP/presence-store"
mkdir -m 0700 "$ST_DIR"
/usr/bin/cp "$ST_WITHOUT_PRESENCE/state.json" "$ST_DIR/state.json"
st_seal
st_config on
st_policy "$SK_TMP/presence.json" true
st_transition presence --policy "$SK_TMP/presence.json" --role authority-admin --role authorizer --role resolver
sk_sign SK-WAR-0009 "Plain Signer"
SK_CHECK=$(st_war check)
if [[ $SK_STATUS -ne 0 ]] && grep -q 'ERROR sign.presence-required .*by the store' <<<"$SK_OUT" && sk_nothing SK-WAR-0009 \
    && grep -q 'ERROR policy.unprotected-divergence .*require_user_presence is false in openwarrant.toml and true' <<<"$SK_CHECK"; then
    sk_ok "the store requires presence" "an ordinary key refused though openwarrant.toml says false"
else
    sk_fail "the store requires presence" "exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT" | head -1)"
fi

# Proposed policy relaxation does not authorize its own adoption. A valid
# ordinary signature has no presence observation: UNKNOWN, with no new head.
st_unseal
"$WAR" authority status --store "$ST_DIR" --emit "$SK_TMP/down.cur.json" --unprotected-test-store >/dev/null 2>&1
"$WAR" authority draft --current "$SK_TMP/down.cur.json" --principal plain \
    --policy "$SK_TMP/policy.json" --role authority-admin --role authorizer --role resolver \
    --emit "$SK_TMP/down.next.json" >/dev/null 2>&1 || st_setup_failed "draft presence downgrade"
"$WAR" authority propose --current "$SK_TMP/down.cur.json" --next "$SK_TMP/down.next.json" \
    --emit "$SK_TMP/down.change.json" >/dev/null 2>&1 || st_setup_failed "propose presence downgrade"
ST_PRESENCE_BEFORE=$(st_head)
SK_DOWNGRADE=$("$WAR" authority approve --store "$ST_DIR" --proposal "$SK_TMP/down.change.json" --principal plain \
    --key "$SK_TMP/plain.pub" --activate --unprotected-test-store 2>&1); SK_D=$?
ST_PRESENCE_AFTER=$(st_head)
if [[ $SK_D -eq 2 && -n "$ST_PRESENCE_BEFORE" && "$ST_PRESENCE_BEFORE" == "$ST_PRESENCE_AFTER" ]] \
    && grep -q 'UNKNOWN authority-user-presence-unknown' <<<"$SK_DOWNGRADE"; then
    sk_ok "ordinary signature cannot relax required presence" "UNKNOWN, unchanged authority head"
else
    sk_fail "ordinary signature cannot relax required presence" "exit $SK_D, head $ST_PRESENCE_BEFORE → $ST_PRESENCE_AFTER"
fi
st_seal
ST_DIR=$ST_WITHOUT_PRESENCE
unset ST_WITHOUT_PRESENCE
st_config on

# The store removes the human's grant: the next act is refused at the new head.
st_transition revoke --policy "$SK_TMP/policy.json" --role authority-admin --role resolver
sk_sign SK-WAR-0010 "Plain Signer"
SK_CHECK=$(st_war check)
if [[ $SK_STATUS -ne 0 ]] && grep -q 'ERROR authority.not-granted .*no `authorizer` role' <<<"$SK_OUT" && sk_nothing SK-WAR-0010; then
    sk_ok "a grant removed, the next act refused" "authority.not-granted at the new head; nothing recorded"
else
    sk_fail "a grant removed, the next act refused" "exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT" | head -1)"
fi
# Observed and reported, not a verdict on it: the read path judges every
# record at the CURRENT head, so the act signed before the revocation now
# reads not-granted too (a decision for the owner; docs/AUTHENTICATION.md).
if grep -q 'ERROR authority.not-granted .*SK-WAR-0008' <<<"$SK_CHECK"; then
    sk_ok "revocation reaches the read path" "the earlier act reads authority.not-granted at the new head"
else
    sk_fail "revocation reaches the read path" "$(grep -E 'SK-WAR-0008' <<<"$SK_CHECK" | grep -E '^(ERROR|PASS) authority' | head -1)"
fi
st_transition regrant --policy "$SK_TMP/policy.json" --role authority-admin --role authorizer --role resolver

# Fail closed: a store the reader can write, one it cannot read, and a test
# store presented as a protected one each give authority.verify-unavailable;
# nothing falls back to roles.toml, and nothing is recorded.
st_fail_closed() { # label, why-pattern
    local check
    check=$(st_war check)
    sk_sign SK-WAR-0011 "Plain Signer"
    if grep -q "^ERROR authority.verify-unavailable .*$2" <<<"$check" \
        && grep -q 'ERROR authority.verify-unavailable .*SK-WAR-0008' <<<"$check" \
        && [[ $SK_STATUS -ne 0 ]] && grep -q 'ERROR authority.verify-unavailable' <<<"$SK_OUT" && sk_nothing SK-WAR-0011; then
        sk_ok "$1" "authority.verify-unavailable on read and sign; nothing recorded"
    else
        sk_fail "$1" "$(grep -E '^ERROR authority.verify' <<<"$check" | head -1) | sign exit $SK_STATUS: $(grep -E '^ERROR' <<<"$SK_OUT" | head -1)"
    fi
}
chmod 0700 "$ST_DIR"
st_fail_closed "a store the reader can write" "writable-by-reader"
chmod 0500 "$ST_DIR"
chmod 0000 "$ST_DIR/state.json"
st_fail_closed "an unreadable store" "could not be read"
chmod 0400 "$ST_DIR/state.json"
st_config on protected
st_fail_closed "a test store claimed as protected" "mode-mismatch"

# Rollback (the work order's): unset the store and the legacy path returns,
# warned — the forged authorization verifies again, which is the point.
st_config off
SK_CHECK=$(st_war check)
if [[ $(grep -c '^WARN authority.unprotected' <<<"$SK_CHECK") -eq 1 ]] \
    && grep -q 'PASS authority.signed .*SK-WAR-0006: signed by mallory' <<<"$SK_CHECK"; then
    sk_ok "the store unset, the register back" "authority.unprotected; the legacy path as before"
else
    sk_fail "the store unset, the register back" "$(grep -E 'SK-WAR-0006' <<<"$SK_CHECK" | grep -E '^(ERROR|PASS) authority' | head -1)"
fi

sk_teardown
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
