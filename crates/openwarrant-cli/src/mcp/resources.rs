// SPDX-License-Identifier: AGPL-3.0-or-later
//! MCP resources: read-only views an agent can pull without a tool call.
//!
//! `warrant://<alias>` (the compiled WAR.md view), `warrant://<alias>/status`,
//! `warrant://<alias>/journal`, `status://corpus` (CORPUS_STATUS.json),
//! `sas://current`, `pins://all`, `next://`.

use rmcp::model::{
    ErrorData as McpError, ListResourcesResult, ReadResourceResult, Resource, ResourceContents,
};

use crate::repo::{RepoError, Repository};

fn resource(uri: &str, name: &str, description: &str, mime: &str) -> Resource {
    let mut r = Resource::new(uri, name).with_description(description);
    r.mime_type = Some(mime.to_owned());
    r
}

fn internal(e: RepoError) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

pub fn list(repo: &Repository) -> Result<ListResourcesResult, McpError> {
    let mut resources = vec![
        resource(
            "status://corpus",
            "corpus status",
            "oh.war/corpus-status/v1: every Warrant's rung, every Objective, what is next",
            "application/json",
        ),
        resource(
            "sas://current",
            "SAS status",
            "the accepted SAS revision and what pins to it",
            "text/plain",
        ),
        resource(
            "pins://all",
            "pinned files",
            "oh.war/pins/v1: files an agent may not edit, with the Warrant that pins each",
            "application/json",
        ),
        resource(
            "next://",
            "next action",
            "oh.war/next/v1: whose act comes next (agent or human) and the command for it",
            "application/json",
        ),
    ];
    for dir in repo.warrant_dirs().map_err(internal)? {
        let Ok(loaded) = repo.load_warrant(&dir) else {
            continue;
        };
        let alias = loaded.alias();
        resources.push(resource(
            &format!("warrant://{alias}"),
            &alias,
            "the compiled Warrant (WAR.md view)",
            "text/markdown",
        ));
        resources.push(resource(
            &format!("warrant://{alias}/status"),
            &format!("{alias} status"),
            "the Warrant's status view",
            "text/plain",
        ));
        resources.push(resource(
            &format!("warrant://{alias}/journal"),
            &format!("{alias} journal"),
            "the Warrant's journal, rendered",
            "text/plain",
        ));
    }
    Ok(ListResourcesResult {
        resources,
        ..Default::default()
    })
}

pub fn read(repo: &Repository, uri: &str) -> Result<ReadResourceResult, McpError> {
    // Resources return the record's OWN schema as its payload (a corpus
    // status, a pins document, a next document, a rendered view), not the
    // report envelope tools answer with; `sas://current` has no record of its
    // own and returns the `sas status` report.
    let text = |t: String, mime: &str| {
        Ok(ReadResourceResult::new(vec![
            ResourceContents::TextResourceContents {
                uri: uri.to_owned(),
                mime_type: Some(mime.to_owned()),
                text: t,
                meta: None,
            },
        ]))
    };
    if let Some(rest) = uri.strip_prefix("warrant://") {
        let (alias, view) = match rest.split_once('/') {
            Some((a, "status")) => (a, "status"),
            Some((a, "journal")) => (a, "journal"),
            Some((_, other)) => {
                return Err(McpError::resource_not_found(
                    format!("no view {other:?}; views are status and journal"),
                    None,
                ));
            }
            None => (rest, "war"),
        };
        let rendered = if view == "journal" {
            crate::journal_cmd::show(repo, alias)
        } else {
            crate::show::run(repo, alias, view)
        }
        .map_err(|e| McpError::resource_not_found(e.to_string(), None))?;
        let mime = if view == "war" {
            "text/markdown"
        } else {
            "text/plain"
        };
        return text(rendered, mime);
    }
    match uri {
        "status://corpus" => {
            let (_, json) = crate::status::corpus_status_json(repo).map_err(internal)?;
            text(json, "application/json")
        }
        "sas://current" => {
            let report = crate::sas::status(repo).map_err(internal)?;
            text(
                crate::output::envelope("sas.status", &report, None),
                "application/json",
            )
        }
        "pins://all" => {
            let pins = crate::pins::list(repo, false).map_err(internal)?;
            text(
                serde_json::to_string_pretty(&pins).unwrap_or_default(),
                "application/json",
            )
        }
        "next://" => {
            let next = crate::next::run(repo).map_err(internal)?;
            text(
                serde_json::to_string_pretty(&next).unwrap_or_default(),
                "application/json",
            )
        }
        other => Err(McpError::resource_not_found(
            format!("unknown resource {other:?}"),
            None,
        )),
    }
}
