#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Compile main's views and commit them to main: the publish step of
# .github/workflows/views.yml (owner decision, 2026-11-07). Pull requests
# carry masters only; this is the one writer of views on main.
#
#   VIEWS_DEPLOY_KEY   the deploy key's private half. Empty: report
#                      "views: not published (VIEWS_DEPLOY_KEY not configured)"
#                      and succeed without pushing.
#   VIEWS_REMOTE       where to push (default git@github.com:$GITHUB_REPOSITORY.git).
#                      Set it to a local path to exercise this without a key,
#                      as plant 240 does; the workflow never sets it.
#   VIEWS_BRANCH       default main.  WAR  default target/debug/war.
#   VIEWS_BUILD        rebuilds WAR for each attempt (default
#                      `cargo build --locked -p openwarrant-cli`).
#   VIEWS_ATTEMPTS     default 4.
#
# It stages only the paths conformance/views.py lists as views, refuses when a
# compile changed anything else, commits as one fixed identity, and pushes a
# fast-forward (never --force). When main moved meanwhile the push is refused,
# so it fetches, rebuilds, recompiles and tries again.
set -euo pipefail

NAME="openwarrant-views"
EMAIL="views@openwarrant.invalid"
BRANCH=${VIEWS_BRANCH:-main}
WAR=${WAR:-target/debug/war}
BUILD=${VIEWS_BUILD:-cargo build --locked -p openwarrant-cli}
ATTEMPTS=${VIEWS_ATTEMPTS:-4}
KEY=${VIEWS_DEPLOY_KEY:-}

if [[ -z "${VIEWS_REMOTE:-}" ]]; then
    if [[ -z "$KEY" ]]; then
        echo "views: not published (VIEWS_DEPLOY_KEY not configured)"
        exit 0
    fi
    REMOTE="git@github.com:${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is unset}.git"
else
    REMOTE=$VIEWS_REMOTE
fi

WORK=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/ow-views.XXXXXX")
[[ -n "$WORK" && -d "$WORK" ]] || exit 2
trap '[[ "$WORK" == */ow-views.* ]] && /usr/bin/rm -rf -- "$WORK"' EXIT
if [[ -n "$KEY" ]]; then
    (umask 077 && printf '%s\n' "$KEY" > "$WORK/key")
    # GitHub's published host key, pinned: no trust on first use.
    printf '%s\n' 'github.com ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl' > "$WORK/known_hosts"
    export GIT_SSH_COMMAND="ssh -i $WORK/key -o IdentitiesOnly=yes -o IdentityAgent=none -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$WORK/known_hosts"
fi
unset SSH_AUTH_SOCK SSH_AGENT_PID

for attempt in $(seq 1 "$ATTEMPTS"); do
    git fetch -q "$REMOTE" "refs/heads/$BRANCH"
    git checkout -q -f --detach FETCH_HEAD
    TIP=$(git rev-parse HEAD)
    if [[ "$(git log -1 --format=%ae HEAD)" == "$EMAIL" ]]; then
        echo "views: $BRANCH's tip ${TIP:0:12} is already a views commit; nothing to do"
        exit 0
    fi
    [[ -z "$(git status --porcelain --untracked-files=all)" ]] \
        || { echo "views: the checkout is not clean before compiling; refusing"; git status --short | head -20; exit 1; }
    $BUILD
    "$WAR" compile >"$WORK/compile.log" 2>&1 \
        || { echo "views: war compile failed"; tail -n 30 "$WORK/compile.log"; exit 1; }
    # Human numbers: each id main has not numbered yet, appended (never renumbered).
    python3 conformance/views.py number --commit "$TIP"
    if ! python3 conformance/views.py changed >"$WORK/paths"; then
        echo "views: refused: the compile changed a path that is not a view (above); nothing pushed"
        exit 1
    fi
    if [[ ! -s "$WORK/paths" ]]; then
        echo "views: up to date at ${TIP:0:12}"
        exit 0
    fi
    git add -A --pathspec-from-file="$WORK/paths"
    # Exactly the listed paths, and nothing left over.
    if ! diff <(sort -u "$WORK/paths") <(git diff --cached --name-only --no-renames | sort -u) >/dev/null \
        || ! git diff --quiet || [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
        echo "views: refused: what is staged is not exactly the listed views; nothing pushed"
        git status --short | head -20
        exit 1
    fi
    git -c user.name="$NAME" -c user.email="$EMAIL" -c commit.gpgsign=false \
        commit -q --no-verify -m "views: compile main" -m "Written by .github/workflows/views.yml from ${TIP}. Views are derived; edit the masters."
    if git push -q "$REMOTE" "HEAD:refs/heads/$BRANCH"; then
        echo "views: published $(wc -l <"$WORK/paths") view(s) onto ${TIP:0:12}"
        exit 0
    fi
    echo "views: attempt $attempt refused (did $BRANCH move?); fetching again"
    sleep $((attempt * 5))
done
echo "views: not published after $ATTEMPTS attempts; $BRANCH kept moving"
exit 1
