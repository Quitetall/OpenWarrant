# Restore the five missing SPDX headers found by the repository gate

The gate on 91f912aa reported missing headers in amendment_id.rs, init/guided.rs, remedy.rs, sas_repin.rs and tui/mod.rs. All five are already missing on unchanged base 1dcbfd79. No resolved delivery pins these paths. This is license metadata repair, not a change to authority, semantics or signed records.
