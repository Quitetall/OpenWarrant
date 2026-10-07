// SPDX-License-Identifier: Apache-2.0
//! `war progress --serve` is `war ui` opened at its Progress page
//! (OW-WAR-0116). The read-only server that lived here allowed inline script
//! and style and carried no session token; the web UI replaces it with one
//! hardened loopback server (`crate::webui`), so there is one set of
//! controls to test rather than two.
use super::*;

pub(super) fn serve(
    repo: Repository,
    port: u16,
    _interval: u64,
    mode: crate::output::Mode,
) -> Result<(), RepoError> {
    crate::webui::run(repo.root, port, "progress", None, mode).map(|_| ())
}
