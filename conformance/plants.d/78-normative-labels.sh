# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# A repeatable extraction can still drop or mislabel a normative rule.
NP_SAS_FILE=$(ls docs/sas/*.md | head -1)

plant "a numbered rule dropped by extraction" "sas-normative.section-dropped" "8-bis. Added rules" 2 \
    "printf '\n## 8-bis. Added rules\n\nThe worker SHALL preserve evidence.\n' >> \"$NP_SAS_FILE\"" \
    --generated

plant "a subsection rule mislabelled as its parent" "sas-normative.section-dropped" "8-bis.1 Added detail" 2 \
    "printf '\n## 8A. Added rules\n\n### 8-bis.1 Added detail\n\nThe worker SHALL preserve evidence.\n' >> \"$NP_SAS_FILE\"" \
    --generated
