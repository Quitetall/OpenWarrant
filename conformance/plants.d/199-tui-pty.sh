# shellcheck shell=bash
# t-3c16c W5 — Mission Control, driven as a person drives it: the real `war`
# on a real pseudo-terminal, a throwaway repository, two worktrees and live
# claims. No TestBackend and no `--render`: bytes go in at the keyboard and the
# screen is what the terminal was told to draw. The agent is a stub, so this
# calls no model and CI needs neither an agent nor Herdr. Python's standard
# library is all it takes.
#
# Session variables are cleared: a session named in the environment hides how an
# unnamed terminal holds a claim. Each claim beside its refusal:
#
# - What Watch shows is what `war status`, `war view tickets`, `war plan
#   questions` and `git worktree list` say, and it follows them with no keypress.
# - Create makes the ticket. Dispatching it before it is committed is refused by
#   name and leaves no claim and no worktree; committed, it starts detached in a
#   worktree of its own, with the claim's actor, the node and the claim id.
# - A scope overlapping another worktree's shows as a collision.
# - An answer from nobody the register names is refused, and one from the human
#   it names is recorded.
# - An empty note is refused. A note on held work lands in the holder's worktree,
#   where `war view show` reads it, and not in this checkout's journal.
# - The agent's `war done` releases the claim on screen, and the item closes in
#   the records once its branch is merged.
# - With `allowed_acts = "declared"` the detached log still has somewhere to go.
#   A harness that cannot start says so in that log, and the claim stands.
# - A `claude` harness is offered this terminal first, and is run with one
#   argument, the brief, in its worktree. `claude` here is a stub on PATH: this
#   proves the launch, and says nothing of what a real session then does.
# - `q` leaves the alternate screen and exits 0.

echo "== Mission Control on a real terminal (t-3c16c W5) =="
TP_TMP=$(mktemp -d)
TP_WAR="$REPO_ROOT/${WAR#./}"
cat >"$TP_TMP/drive.py" <<'DRIVE'
import fcntl, json, os, pty, re, select, signal, struct, subprocess, sys, termios, time

WAR, TMP = sys.argv[1], sys.argv[2]
R = TMP + "/repo"
STUB = TMP + "/stub"
os.makedirs(R)
os.makedirs(STUB)
GONE = ("OPENWARRANT_SESSION_ID", "KATANA_SESSION_ID", "CODEX_THREAD_ID", "CODEX_SESSION_ID",
        "CLAUDE_CODE_SESSION_ID", "SSH_AUTH_SOCK", "SSH_AGENT_PID", "HERDR_SESSION", "HERDR_ENV",
        "HERDR_PANE_ID", "HERDR_TAB_ID", "HERDR_WORKSPACE_ID", "GIT_DIR", "GIT_WORK_TREE")
ENV = {k: v for k, v in os.environ.items() if k not in GONE}
ENV.update(TERM="xterm-256color", HERDR_SOCKET_PATH=TMP + "/no.sock", OPENWARRANT_NO_PROJECTS="1",
           OPENWARRANT_ACTOR="plant-owner", OPENWARRANT_NO_UPDATE_CHECK="1", STUB_DIR=STUB,
           STUB_WAR=WAR, GIT_AUTHOR_NAME="p", GIT_AUTHOR_EMAIL="p@e", GIT_COMMITTER_NAME="p",
           GIT_COMMITTER_EMAIL="p@e")
FAILED = [0]

def ok(name, detail):
    print("ok    %-34s %s" % (name, detail), flush=True)

def fail(name, detail):
    FAILED[0] += 1
    print("FAIL  %-34s %s" % (name, " ".join(str(detail).split())[:380]), flush=True)

def claim(name, held, detail, why):
    (ok if held else fail)(name, detail if held else why)

def sh(cmd, cwd=R):
    return subprocess.run(cmd, shell=True, cwd=cwd, env=ENV, capture_output=True, text=True, timeout=30)

def war(*args, cwd=R):
    return subprocess.run([WAR, *args], cwd=cwd, env=ENV, capture_output=True, text=True, timeout=60)

def result(*args, cwd=R):
    return json.loads(war("--json", *args, cwd=cwd).stdout)["result"]


class Term:
    """A pty, the child on it, and the screen it drew."""

    def __init__(self, argv, rows=45, cols=150):
        self.rows, self.cols = rows, cols
        self.grid = [[" "] * cols for _ in range(rows)]
        self.r = self.c = 0
        self.tail = b""
        self.alt = self.ever_alt = False
        self.code = None
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.chdir(R)
            os.execve(argv[0], argv, ENV)
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
        os.kill(self.pid, signal.SIGWINCH)

    def csi(self, q):
        final, args = q[-1], q[:-1]
        nums = [int(x) if x.isdigit() else 0 for x in args.lstrip("?").split(";")] if args else []
        p = lambda i, d: nums[i] if i < len(nums) and nums[i] else d
        if final in "Hf":
            self.r, self.c = p(0, 1) - 1, p(1, 1) - 1
        elif final == "J" and p(0, 0) in (2, 3):
            self.grid = [[" "] * self.cols for _ in range(self.rows)]
        elif final == "K":
            self.grid[self.r][self.c:] = [" "] * (self.cols - self.c)
        elif final == "C":
            self.c += p(0, 1)
        elif final == "n" and args == "6":  # a terminal answers where its cursor is
            os.write(self.fd, b"\x1b[%d;%dR" % (self.r + 1, self.c + 1))
        elif final in "hl" and args.startswith("?1049"):
            self.alt = final == "h"
            self.ever_alt = self.ever_alt or self.alt

    def feed(self, data):
        s = self.tail + data
        i, n = 0, len(s)
        while i < n:
            b = s[i]
            if b == 0x1b:
                if i + 1 >= n:
                    break
                if s[i + 1] == 0x5b:
                    m = re.compile(rb"[0-9;?<>=:]*[ -/]*[@-~]").match(s, i + 2)
                    if not m:
                        break
                    self.csi(m.group().decode("latin1"))
                    i = m.end()
                else:
                    i += 2
            elif b == 0x0d:
                self.c = 0
                i += 1
            elif b == 0x0a:
                self.r = min(self.r + 1, self.rows - 1)
                i += 1
            elif b < 0x20:
                i += 1
            else:
                size = 1 if b < 0x80 else 2 if b < 0xe0 else 3 if b < 0xf0 else 4
                if i + size > n:
                    break
                if self.r < self.rows and self.c < self.cols:
                    self.grid[self.r][self.c] = s[i:i + size].decode("utf-8", "replace")
                self.c += 1
                i += size
        self.tail = s[i:]

    def pump(self, wait=0.05):
        if select.select([self.fd], [], [], wait)[0]:
            try:
                data = os.read(self.fd, 65536)
            except OSError:
                return False
            if not data:
                return False
            self.feed(data)
        return True

    def screen(self):
        return "\n".join("".join(r).rstrip() for r in self.grid)

    def wait(self, pattern, seconds=20):
        end, rx = time.time() + seconds, re.compile(pattern)
        while time.time() < end:
            if rx.search(self.screen()):
                return True
            if not self.pump():
                break
        return bool(rx.search(self.screen()))

    def settle(self, quiet=0.25, seconds=4):
        end, at, last = time.time() + seconds, time.time(), self.screen()
        while time.time() < end and time.time() - at < quiet:
            self.pump()
            now = self.screen()
            if now != last:
                last, at = now, time.time()

    def keys(self, *sequence):
        for k in sequence:
            os.write(self.fd, k.encode())
            self.settle()

    def type(self, text):
        for ch in text:
            os.write(self.fd, ch.encode())
            self.pump(0.01)
        self.settle()

    def quit(self):
        os.write(self.fd, b"q")
        end = time.time() + 10
        while time.time() < end and self.code is None:
            self.pump()
            pid, status = os.waitpid(self.pid, os.WNOHANG)
            if pid:
                self.code = os.waitstatus_to_exitcode(status)
        while self.pump(0.05):
            pass
        return self.code

    def kill(self):
        if self.code is None:
            try:
                os.kill(self.pid, signal.SIGKILL)
                os.waitpid(self.pid, 0)
            except OSError:
                pass


ESC, ENTER, DEL = "\x1b", "\r", "\x7f"
REPO_TOML = R + "/openwarrant.toml"


def worktrees():
    return len([l for l in sh("git worktree list --porcelain").stdout.splitlines() if l.startswith("worktree ")])


def header(t):
    """The two count lines, as the screen has them."""
    text = t.screen()
    m = re.search(r"CLAIMS (\d+) \((\d+) live.*WORKTREES (\d+)  SIGN (\d+)", text)
    w = re.search(r"WARRANTS (\d+)/(\d+) resolved", text)
    i = re.search(r"ITEMS (\d+) open, (\d+) active, (\d+) done", text)
    if not (m and w and i):
        return None
    return {"claims": int(m[1]), "worktrees": int(m[3]), "sign": int(m[4]), "resolved": int(w[1]),
            "warrants": int(w[2]), "open": int(i[1]), "active": int(i[2]), "done": int(i[3])}


def records():
    """The same numbers, from the commands, by no code path of the app's."""
    status = result("status")
    tickets = result("view", "tickets")["tickets"]
    state = lambda s: sum(1 for x in tickets if x["state"] == s)
    return {"claims": sum(len(x.get("claims", [])) for x in tickets), "worktrees": worktrees(),
            "sign": len(result("sign", "--list")["acts"]), "warrants": len(status["warrants"]),
            "resolved": sum(1 for x in status["warrants"] if x.get("rung") == "resolved"),
            "tickets_open": state("open"), "tickets_active": state("in_progress"),
            "tickets_done": state("done")}


def same(t, name):
    """Watch agrees with the commands, with no key pressed to make it."""
    want, got = None, None
    for _ in range(40):
        want = records()
        t.pump(0.25)
        got = header(t)
        if got:
            # A Warrant is an open item too; tickets are what these commands count here.
            items = (got["open"] - len(result("status")["warrants"]), got["active"], got["done"])
            if all(got[k] == want[k] for k in ("claims", "worktrees", "sign", "warrants", "resolved")) \
                    and items == (want["tickets_open"], want["tickets_active"], want["tickets_done"]):
                ok(name, "%d claim(s), %d worktree(s), %d to sign, %d Warrant(s), items %d/%d/%d" % (
                    got["claims"], got["worktrees"], got["sign"], got["warrants"], *items))
                return True
        time.sleep(0.25)
    fail(name, "screen %s, records %s" % (got, want))
    return False


def highlight(t, text):
    """Step the panels, then Work's rows, until the command line is about `text`."""
    for _ in range(6):
        if re.search(r"\$ war view show", t.screen()):
            break
        t.keys("l")
    for _ in range(12):
        if re.search(r"\$ war [^\n]*" + re.escape(text), t.screen()):
            return True
        t.keys("j")
    return False


def dispatch(t, scope):
    """Executor, scope, a new worktree, detached."""
    t.keys("d", ENTER)
    t.type(scope)
    t.keys(ENTER, ENTER, ENTER)


def wait_file(path, seconds=20):
    end = time.time() + seconds
    while time.time() < end and not os.path.exists(path):
        time.sleep(0.05)
    return os.path.exists(path)


# ---- a throwaway repository ------------------------------------------------
sh("git init -q -b main .")
if war("init").returncode != 0:
    print("PLANT SETUP FAILED: war init", file=sys.stderr)
    sys.exit(9)
open(STUB + "/stub-agent", "w").write("""#!/bin/sh
# An agent that calls no model: it reports, waits to be told, and closes its item.
node=$1
slug=$(printf %s "$node" | tr / _)
env | grep -E '^(OPENWARRANT_ACTOR|OPENWARRANT_CLAIM_ID|WAR_GO_NODE)=' > "$STUB_DIR/$slug.env"
pwd > "$STUB_DIR/$slug.pwd"
tries=0
while [ ! -e "$STUB_DIR/finish" ] && [ ! -e "$STUB_DIR/$slug.finish" ] && [ $tries -lt 600 ]; do
    sleep 0.1; tries=$((tries + 1))
done
"$STUB_WAR" done "$node" --note "the stub is done"
echo "$?" > "$STUB_DIR/$slug.exit"
""")
os.chmod(STUB + "/stub-agent", 0o755)
# `[go] harness = "claude"` runs `claude` by name: this one, first on PATH. It
# records its argv and exits. No model is called, and the brief goes nowhere.
os.makedirs(STUB + "/bin")
open(STUB + "/bin/claude", "w").write("""#!/bin/sh
{ printf '%s\\n' "$#"; pwd; printf '%s\\n' "$@"; } > "$STUB_DIR/claude.argv"
""")
os.chmod(STUB + "/bin/claude", 0o755)
ENV["PATH"] = STUB + "/bin:" + ENV["PATH"]
with open(REPO_TOML, "a") as f:
    f.write('\n[go]\nharness = "generic"\nstart_argv = ["%s/stub-agent", "{node}"]\n' % STUB)
os.makedirs(R + "/docs/authority", exist_ok=True)
open(R + "/docs/authority/roles.toml", "w").write("""[[assignment]]
actor = "Plant Owner"
actor_kind = "human"
roles = ["authorizer", "resolver"]
assigned_by = "conformance/plants.d/199-tui-pty.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while this plant runs."
""")
held = result("create", "Work already running", "--item", "the first one")["id"]
later = result("create", "Declared acts", "--item", "the second one")["id"]
broken = result("create", "A harness that is not there", "--item", "the third one")["id"]
real = result("create", "The Claude Code preset", "--item", "the fourth one")["id"]
war("new", "Stage work")
alias = sorted(os.listdir(R + "/docs/warrants"))[0]
war("plan", "ask", alias, "STAGE-001", "Which queue?", "--recommend", "the fast one", "--blocking")
sh("git add -A && git commit -qm setup")
start = result("start", held, "--worktree", "--print", "--as", "other-agent", "--paths", "src/**")
if not start.get("worktree"):
    print("PLANT SETUP FAILED: the first worktree", file=sys.stderr)
    sys.exit(9)

t = Term([WAR])
try:
    if not t.wait(r"CLAIMS \d", 30):
        print("PLANT SETUP FAILED: no Watch on the terminal\n" + t.screen()[:800], file=sys.stderr)
        sys.exit(9)
    t.settle(0.5)
    claim("the real app on a real terminal", t.ever_alt and "herdr: not running" in t.screen(),
          "alternate screen, Watch, no Herdr", t.screen()[:300])
    claim("no usage reads UNKNOWN", "TOKENS UNKNOWN" in t.screen(), "TOKENS UNKNOWN", t.screen()[:300])
    same(t, "Watch matches the commands")

    # ---- create ------------------------------------------------------------
    t.keys("2", "c")
    t.type("Add a fast path")
    t.keys(ENTER)
    t.type("Fuzz it")
    t.keys(ENTER)
    t.wait(r"exit 0|\[exit", 15)
    t.settle()
    made = [x for x in result("view", "tickets")["tickets"] if x["title"] == "Add a fast path"]
    claim("Create makes the ticket", len(made) == 1 and made[0]["total"] == 1, made and made[0]["id"], t.screen()[:400])
    new = made[0]["id"] if made else "t-none"
    t.keys(ESC)
    t.wait("Fuzz it", 10)

    # ---- dispatch: refused before the commit, started after ----------------
    t.keys("r")
    t.settle(0.6)
    # Critical path first: what is uncommitted may not be first; the filter finds it.
    t.keys("/")
    t.type("Fuzz")
    t.keys(ENTER)
    before = worktrees()
    dispatch(t, "src/parser/**")
    t.wait("go.start-uncommitted", 15)
    refused = "go.start-uncommitted" in t.screen()
    claimed = any(x["id"] == new and x.get("claims") for x in result("view", "tickets")["tickets"])
    claim("uncommitted work is refused", refused and worktrees() == before and not claimed,
          "go.start-uncommitted; no claim, no worktree", t.screen()[:400])
    t.keys(ESC)
    sh("git add -A && git commit -qm ticket")
    dispatch(t, "src/parser/**")
    t.wait("started detached", 20)
    claim("Dispatch starts it detached", "started detached" in t.screen(), "claimed, worktree, pid, log", t.screen()[:500])
    t.keys(ESC, ESC)
    node = [x for x in result("view", "tickets")["tickets"] if x["id"] == new][0]
    item = result("view", "show", new)["items"][0]["id"]
    slug = "%s_%s" % (new, item)
    tree = R + "/.openwarrant/worktrees/%s--%s" % (new, item)
    started = wait_file("%s/%s.env" % (STUB, slug))
    seen = open("%s/%s.env" % (STUB, slug)).read() if started else ""
    wd = open("%s/%s.pwd" % (STUB, slug)).read().strip() if started else ""
    claim("the agent starts in its worktree", wd == os.path.realpath(tree) and "OPENWARRANT_ACTOR=stub-agent" in seen
          and "WAR_GO_NODE=%s/%s" % (new, item) in seen and "OPENWARRANT_CLAIM_ID=0" in seen,
          "stub-agent, its node and claim id", "%s: %s" % (wd, seen))
    logged = os.path.exists(tree + "/.openwarrant/start.log")
    dirt = sh("git status --short", cwd=tree).stdout
    claim("the log is no part of the work", logged and ".openwarrant" not in dirt, ".openwarrant/start.log, ignored", dirt)

    # ---- watch -------------------------------------------------------------
    t.keys("1")
    same(t, "Watch follows the dispatch")
    t.wait("COLLISIONS 1", 10)
    claim("an overlap shows as a collision", "COLLISIONS 1 " in t.screen()
          and len(re.findall(r"COLLISION +│", t.screen())) == 2, "src/parser/** beside src/**", t.screen()[:300])

    # ---- answer ------------------------------------------------------------
    t.keys("2", "h", "h", "h", "l", "l")
    t.keys("a", ENTER)
    t.keys(*[DEL] * len("Plant Owner"))
    t.type("Ghost")
    t.keys(ENTER)
    t.wait(r"exit [1-9]|refus|not a human|role", 15)
    t.settle()
    q = result("plan", "questions")
    claim("an answer from nobody is refused", q["open"] == 1 and q["answered"] == 0, "the register decides; still open", q)
    t.keys(ESC, "a", ENTER)
    t.keys(ENTER)
    t.wait(r"\[exit 0\]", 15)
    q = result("plan", "questions")
    claim("an answer is recorded", q["answered"] == 1 and q["open"] == 0 and "the fast one" in t.screen() + json.dumps(q),
          "the recommendation, as Plant Owner", json.dumps(q)[:300] + t.screen()[:300])
    t.keys(ESC)
    t.wait("no open questions", 10)

    # ---- a note reaches the holder -----------------------------------------
    t.keys("1")
    if not highlight(t, new):
        fail("a note reaches the holder", "no row for %s: %s" % (new, t.screen()[-400:]))
    t.keys("a", ENTER)
    claim("an empty note is refused", "empty note says nothing" in t.screen(), "the form stays open", t.screen()[-300:])
    t.type("use the fuzz corpus")
    t.keys(ENTER)
    t.wait(r"\[exit 0\]", 15)
    shown = war("view", "show", new, cwd=tree).stdout
    here = open("%s/docs/tickets/%s/journal.jsonl" % (R, new)).read()
    claim("a note reaches the holder", "use the fuzz corpus" in shown and "fuzz corpus" not in here,
          "in the worktree, read by `war view show`", shown[:300])
    t.keys(ESC)

    # ---- close -------------------------------------------------------------
    open("%s/%s.finish" % (STUB, slug), "w").close()
    finished = wait_file("%s/%s.exit" % (STUB, slug))
    code = open("%s/%s.exit" % (STUB, slug)).read().strip() if finished else "none"
    log = open(tree + "/.openwarrant/start.log").read()
    claim("the agent closes its item", code == "0", "war done in its worktree, exit 0", "exit %s: %s" % (code, log[-300:]))
    t.wait(r"CLAIMS 1 ", 15)
    claim("the claim leaves with no keypress", "CLAIMS 1 " in t.screen(), "one holder left", t.screen()[:300])
    # What its branch closed reaches this checkout when it merges.
    sh("git add -A && git commit -qm work", cwd=tree)
    merged = sh("git merge -q --no-edit war-go/%s--%s" % (new, item))
    state = [x for x in result("view", "tickets")["tickets"] if x["id"] == new][0]["state"]
    claim("merged, the records say done", merged.returncode == 0 and state == "done", "done", merged.stderr + state)
    t.keys("r")
    same(t, "Watch follows the close")

    # ---- declared acts, and a harness that is not there --------------------
    text = open(REPO_TOML).read().replace('harness = "generic"', 'harness = "generic"\nallowed_acts = "declared"')
    open(REPO_TOML, "w").write(text)
    t.keys("r")
    t.settle(0.6)
    t.keys("2", "/")
    t.type("second one")
    t.keys(ENTER)
    dispatch(t, "")
    t.wait("started detached|No such file|start.log", 20)
    claim("declared acts still log", "started detached" in t.screen(), "no session files, a log all the same", t.screen()[:500])
    t.keys(ESC, ESC)
    item2 = result("view", "show", later)["items"][0]["id"]
    claim("declared acts start the agent", wait_file("%s/%s_%s.env" % (STUB, later, item2)), "stub-agent ran", "no stub")

    text = open(REPO_TOML).read().replace('"%s/stub-agent"' % STUB, '"%s/no-such-agent"' % STUB)
    open(REPO_TOML, "w").write(text)
    t.keys("r")
    t.settle(0.6)
    t.keys("/")
    t.type("third one")
    t.keys(ENTER)
    dispatch(t, "")
    t.wait("started detached", 20)
    t.keys(ESC, ESC)
    item3 = result("view", "show", broken)["items"][0]["id"]
    log = R + "/.openwarrant/worktrees/%s--%s/.openwarrant/start.log" % (broken, item3)
    time.sleep(1)
    said = open(log).read() if os.path.exists(log) else "(no log)"
    stands = [x for x in result("view", "tickets")["tickets"] if x["id"] == broken][0].get("claims")
    claim("a harness not there says so", "no-such-agent" in said and bool(stands), "in the log; the claim stands", said[:300])

    # ---- the launch of a real harness, with no model behind it -------------
    text = re.sub(r'harness = "generic"\nallowed_acts = "declared"\nstart_argv = .*\n', 'harness = "claude"\n',
                  open(REPO_TOML).read())
    open(REPO_TOML, "w").write(text)
    t.keys("r")
    t.settle(0.6)
    t.keys("/")
    t.type("fourth one")
    t.keys(ENTER, "d", ENTER, ENTER, ENTER)
    offered = "this terminal, until the session ends" in t.screen().split("Launch in")[-1].split("\n")[1]
    t.keys(ENTER)
    wrote = wait_file(STUB + "/claude.argv")
    t.settle(0.5)
    t.keys(ESC, ESC)
    item4 = result("view", "show", real)["items"][0]["id"]
    argv = open(STUB + "/claude.argv").read().split("\n", 2) if wrote else ["", "", ""]
    claim("claude gets this terminal first", offered, "a harness that needs one is offered it", t.screen()[:400])
    claim("the launch argv of claude", wrote and argv[0] == "1" and argv[1].endswith("%s--%s" % (real, item4))
          and "Tracked work `%s/%s`, claimed by `claude`" % (real, item4) in argv[2],
          "one argument, the brief, in the worktree", repr(argv)[:300])

    # ---- the terminal comes back ------------------------------------------
    code = t.quit()
    claim("q gives the terminal back", code == 0 and t.ever_alt and not t.alt, "exit 0, alternate screen left", "exit %s, alt %s" % (code, t.alt))
finally:
    t.kill()
    open(STUB + "/finish", "w").close()
    end = time.time() + 10
    while time.time() < end and sh("pgrep -f '%s/stub-agent' | head -1" % STUB).stdout.strip():
        time.sleep(0.1)
sys.exit(1 if FAILED[0] else 0)
DRIVE
TP_OUT=$(timeout 150 python3 "$TP_TMP/drive.py" "$TP_WAR" "$TP_TMP" 2>&1); TP_RC=$?
printf '%s\n' "$TP_OUT" | cut -c1-420
TP_OK=$(grep -c '^ok ' <<<"$TP_OUT")
TP_BAD=$(grep -c '^FAIL ' <<<"$TP_OUT")
PASSED=$((PASSED + TP_OK)); FAILED=$((FAILED + TP_BAD))
if [[ $TP_RC -eq 9 ]]; then
    printf 'PLANT SETUP FAILED: the terminal walk\n' >&2
    exit 9
fi
if [[ $TP_RC -ne 0 && $TP_BAD -eq 0 ]]; then
    printf 'FAIL  %-34s %s\n' "the terminal walk ended" "exit $TP_RC: $(tail -3 <<<"$TP_OUT" | tr '\n' ' ')"
    FAILED=$((FAILED + 1))
fi
/usr/bin/rm -rf "$TP_TMP"
unset TP_TMP TP_WAR TP_OUT TP_RC TP_OK TP_BAD
