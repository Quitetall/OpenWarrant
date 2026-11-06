# shellcheck shell=bash
# Rules that live in files resolved Warrants pin, landed as one correction
# batch: the roadmap may not claim a resolution (C6), a roadmap ref names this
# program's §98 (C5), a SAS acceptance dated "soon" is refused (B2).

plant "a resolved claim in the roadmap is refused" "roadmap.status-claim" "§56.2" 2 \
    "printf '\n| OW-WAR-0032 | **resolved** 2026-08-20 |\n' >> docs/roadmap/PRODUCTION_ROADMAP.md; assert_present '**resolved**' docs/roadmap/PRODUCTION_ROADMAP.md"

plant "a roadmap ref into another program is refused" "roadmap.wrong-namespace" "XX" 2 \
    "sed -i 's|roadmap://OW-PHASE-6/conformance|roadmap://XX-PHASE-6/conformance|' docs/warrants/OW-WAR-0063/manifest.toml; assert_present 'XX-PHASE-6' docs/warrants/OW-WAR-0063/manifest.toml" \
    OW-WAR-0063

SAS_TMP=$(mktemp -d)
plant_cmd "sas accept refuses effective_time = soon" "sas.effective-time" "RFC 3339" 2 \
    "\"$WAR\" sas propose 0.1.0-draft.9 >/dev/null 2>&1 || { echo 'sas propose 0.1.0-draft.9 failed; the plant cannot run' >&2; exit 9; }; \
     d=\$(grep '^sha256' docs/sas/revisions/0.1.0-draft.9.toml | cut -d'\"' -f2); \
     printf 'schema = \"oh.war/sas-acceptance-response/v1\"\nversion = \"0.1.0-draft.9\"\nsha256 = \"%s\"\naccepted_by = \"Brian Lam\"\nacting_role = \"authorizer\"\nmeaning = \"x\"\neffective_time = \"soon\"\n' \"\$d\" > \"$SAS_TMP/soon.toml\"" \
    sas accept 0.1.0-draft.9 --response "$SAS_TMP/soon.toml"
rm -rf "$SAS_TMP"
