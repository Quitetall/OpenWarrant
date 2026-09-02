---
schema: oh.war/atom/v1
warrant_uuid: 01a060de-a1c9-77f2-962a-74ca52797a34
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS v0.1.0-draft.1, sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.
- §17.5 projections; §59.2 committed generated views verified for drift;
  §77.3 consumers take generated artifacts and do not reimplement WAR
  semantics.
- RQ-075.
- ADR 0179 (CI cost discipline), ADR 0182 (public repository; GitHub-hosted
  runners only).

## Measured on 2026-09-02

- `gh api repos/Quitetall/OpenWarrant/pages` → 404: no Pages site.
- The repository is public, so Pages is free.
- `CORPUS_STATUS.html` links `CORPUS_STATUS.json` and `CORPUS_STATUS.md` by
  relative href, so the three must be published side by side.
- Two workflows exist (`ci.yml`, `release.yml`); every `uses:` in them is
  pinned to a 40-hex SHA with the tag in a trailing comment. Nothing
  checked that.
- `actions/configure-pages` v6.0.0 = `45bfe019…`, `upload-pages-artifact`
  v5.0.0 = `fc324d35…`, `deploy-pages` v5.0.1 = `368f8252…`, resolved
  through the GitHub API on the day.

## Measured during execution

- The first version of the `uses:` matcher stripped the prefix `uses:` and
  so skipped every `- uses:` line — the common form. The gate step reported
  "3 workflow(s): every uses pinned" while checking nothing; the unit test
  that feeds it a tag-pinned action caught it before the step ever ran in
  CI. Recorded because it is the exact failure the step exists to prevent,
  in the step itself.
- External review of the second matcher found three more vacuous passes —
  `{uses: x@v1}`, `"uses": x@v1`, `uses : x@v1` — all forms GitHub's YAML
  parser accepts. OW-ADR-0002 keeps YAML libraries out of this repository,
  so the fix is a matcher that finds every `uses` key on a line rather than
  a parser; each form has a test.

- First deploy, 2026-09-02, from the squash commit of #49 (`3d514c7`):
  run <https://github.com/Quitetall/OpenWarrant/actions/runs/33601820487>,
  success. `sha256sum` of the served `https://quitetall.github.io/OpenWarrant/`
  and of `/CORPUS_STATUS.html` both
  `44e3165a45838e3ec1024c5d0fecda54b0658589af24643aa652d0eedc92eb5f`, equal
  to the committed `docs/warrants/generated/CORPUS_STATUS.html` at that
  commit. `/CORPUS_STATUS.json` parses and lists 56 Warrants. Measured by
  the performer; OBL-003 needs an independent verifier to say the same.

## Assumptions carried in

- Pages is enabled with `build_type = workflow` through the API at merge
  time; the workflow cannot enable it for itself and fails until it is.
- The site is served from `main` only. A pull request does not deploy; the
  drift check on the pull request is the review.
