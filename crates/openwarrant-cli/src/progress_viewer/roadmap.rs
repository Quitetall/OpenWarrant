// SPDX-License-Identifier: Apache-2.0
//! Authored grouping only. Tracker reports remain the source of work state.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Roadmap {
    schema: String,
    title: String,
    nodes: Vec<Node>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Node {
    id: String,
    title: String,
    outcome: String,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    warrants: Vec<String>,
}
pub(super) fn parse(bytes: &[u8], aliases: &BTreeSet<&str>) -> Result<Roadmap, String> {
    let map: Roadmap = serde_json::from_slice(bytes).map_err(|e| format!("Roadmap: {e}"))?;
    if map.schema != "oh.war/roadmap-view/v1"
        || map.title.trim().is_empty()
        || map.nodes.len() > 1024
    {
        return Err("Invalid roadmap schema, title or node limit".into());
    }
    let mut nodes = BTreeMap::new();
    for n in &map.nodes {
        if n.id.is_empty()
            || n.id.len() > 128
            || n.title.trim().is_empty()
            || n.outcome.len() > 4096
        {
            return Err(format!("Invalid roadmap node {}", n.id));
        }
        if nodes.insert(n.id.as_str(), n).is_some() {
            return Err(format!("Duplicate roadmap node {}", n.id));
        }
        for alias in &n.warrants {
            if !aliases.contains(alias.as_str()) {
                return Err(format!(
                    "Roadmap node {} references unknown Warrant {alias}",
                    n.id
                ));
            }
        }
    }
    for n in &map.nodes {
        let mut seen = BTreeSet::new();
        let mut current = n;
        loop {
            if !seen.insert(current.id.as_str()) {
                return Err(format!("Roadmap parent cycle at {}", current.id));
            }
            let Some(parent) = &current.parent else { break };
            current = nodes.get(parent.as_str()).ok_or_else(|| {
                format!("Roadmap node {} has missing parent {parent}", current.id)
            })?;
        }
    }
    Ok(map)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input(nodes: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(
            &serde_json::json!({"schema":"oh.war/roadmap-view/v1","title":"Release","nodes":nodes}),
        )
        .unwrap()
    }
    #[test]
    fn shared_references_are_valid_but_bad_topology_refuses() {
        let aliases = BTreeSet::from(["OW-WAR-0001"]);
        let node = serde_json::json!({"id":"a","title":"Feature","outcome":"Useful","warrants":["OW-WAR-0001"]});
        let mut second = node.clone();
        second["id"] = "b".into();
        second["parent"] = "a".into();
        assert!(parse(&input(serde_json::json!([node, second])), &aliases).is_ok());
        assert!(
            parse(&input(serde_json::json!([node, node])), &aliases)
                .err()
                .unwrap()
                .contains("Duplicate")
        );
        second["parent"] = "b".into();
        assert!(
            parse(&input(serde_json::json!([second])), &aliases)
                .err()
                .unwrap()
                .contains("cycle")
        );
        second["parent"] = "missing".into();
        assert!(
            parse(&input(serde_json::json!([second])), &aliases)
                .err()
                .unwrap()
                .contains("missing parent")
        );
        assert!(
            parse(&input(serde_json::json!([node])), &BTreeSet::new())
                .err()
                .unwrap()
                .contains("unknown Warrant")
        );
    }
}
