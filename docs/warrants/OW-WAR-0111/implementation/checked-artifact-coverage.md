# Artifact coverage from checked declarations

The local exporter now derives artifact coverage from its checked current and
retained historical deliverable declarations. History must be captured. Every
claim must resolve to exact content-addressed bytes before coverage is retained;
otherwise the report identifies unavailable declaration IDs and reasons. Retained
coverage lists the declaration sources, history index, artifact index and bytes.
An empty declaration set retains its inventory; it does not invent artifact bytes.

Offline inspection checks a retained artifact coverage claim against that same
inventory. A caller cannot upgrade an unavailable category by changing only its
coverage label. This is coverage within the selected local source/history boundary;
provider-owned records still need the provider export. Other refs and unavailable
history are not silently included. Overall preservation remains incomplete.

Validation: twelve preservation tests and warnings-denied all-target CLI Clippy
passed with Rust 1.97.1. Tests cover positive historical artifact coverage and a
forged retained label without captured history. Real OW-WAR-0030 export and inspect
passed: its three distinct artifact byte versions are retained; six other required
categories remain unavailable. No assurance or completion disposition is issued.
