# Native acceptance collection

The manual/PR workflow `.github/workflows/phase1-native.yml` runs the existing
complete Phase 1 runner on native Linux and macOS. It writes receipts outside the
checkout so evidence output cannot alter the source inventory being measured.
Both receipts retain Git/build identity, before/after source bytes, test logs,
fixture assignments, host and executable identity, and explicit qualification gaps.
Failures retain their receipt; a missing receipt fails artifact upload.

A green job is not phase acceptance. Before closing the platform gap, inspect both
receipts for current_host_passed, matching source_digest/source_files and Git SHA,
source/binary stability, all required check outcomes and distinct native hosts.
Independent phase review and human qualification remain separate requirements.
This workflow publishes no release and performs no human authorization.

The first run remains pending. No macOS result is claimed by adding this file.

First native run 35296418894: Linux passed; macOS failed only inventory executable-path assertion. macOS temporary paths resolve through /private/var. A Linux directory-symlink reproduction confirmed the same mismatch. Test now compares canonical executable identity and exercises a symlink explicitly; runner behavior and required expectations remain unchanged. Native rerun pending. Failed receipts retained by the original Actions run.
