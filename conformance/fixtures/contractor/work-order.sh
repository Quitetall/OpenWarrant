#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The contractor Work Order fixture (OW-WAR-0140).
#
#   work-order.sh <war> <program-root> <acceptance-authority>
#
# Writes one contractor-profile Warrant into <program-root> with `war new
# --profile contractor`, then answers its four contractor atoms, and prints
# the Warrant's directory. The program must already hold
# profiles/contractor.toml.
#
# NON-BINDING FIXTURE. Everything this writes is invented for a test. It is
# not a contract, not a legal instrument, and not legal, financial or quality
# advice; every party, term, invoice and payment below is a reference to a
# record that does not exist. No legal, finance or QMS decision stands behind
# it (SAS §98; OW-WAR-0140 U-001). The FIXTURE-* tokens are there so a plant
# can prove none of them reaches the technical core's IR.

set -euo pipefail

war="$1"
root="$2"
authority="$3"

out=$("$war" --root "$root" new --profile contractor \
    "Fixture: a contractor Work Order (non-binding)")
alias=$(grep -oE '[A-Z][A-Z0-9-]*-WAR-[0-9]{4}' <<<"$out" | head -1)
dir="$root/docs/warrants/$alias"
[[ -f "$dir/manifest.toml" ]] || { echo "fixture: war new wrote no manifest" >&2; exit 2; }

# Keep the header `war new` wrote (plus one line, if given); replace the body.
answer() { # <file> <extra header line or ""> <body>
    local path="$dir/atoms/$1"
    {
        echo '---'
        awk 'NR == 1 { next } /^---$/ { exit } { print }' "$path"
        if [[ -n "$2" ]]; then echo "$2"; fi
        echo '---'
        echo
        printf '%s\n' "$3"
    } >"$path.new"
    mv "$path.new" "$path"
}

BANNER='> NON-BINDING FIXTURE. Not a contract and not a legal instrument; not
> legal, financial or quality advice. Every record cited below is invented
> for a test and does not exist.'

answer 71-contractor-parties.md "" "# Contractor Parties

$BANNER

## Parties

- The customer: kf://parties/FIXTURE-PARTY-CUSTOMER
- The contractor: kf://parties/FIXTURE-PARTY-CONTRACTOR"

answer 72-contractor-terms.md "" "# Contractor Terms

$BANNER

## Terms, by reference

Each term is the contractual Work Order's, linked and never copied (§22.3):

- authorization scope: kf://work-orders/FIXTURE-WO-0001#scope
- compensation: kf://work-orders/FIXTURE-WO-0001#compensation
- schedule: kf://work-orders/FIXTURE-WO-0001#schedule
- confidentiality: kf://work-orders/FIXTURE-WO-0001#confidentiality
- legal terms: kf://work-orders/FIXTURE-WO-0001#legal"

answer 73-contractor-acceptance.md "acceptance_authority: $authority" "# Contractor Acceptance

$BANNER

## Acceptance authority

The actor this atom's header names accepts by resolving the Warrant with
\`war resolve\`. No other act accepts.

## Acceptance criteria

The obligations in 60-assurance.md, as the resolution judges them."

answer 74-contractor-commercial.md "" "# Contractor Commercial Relations

$BANNER

## Invoice and payment relations

- invoice: kf://finance/invoices/FIXTURE-INV-0001
- payment: kf://finance/payments/FIXTURE-PAY-0001"

printf '%s\n' "$dir"
