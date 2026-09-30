# RC.2 source-set acceptance

Status: proposed local architecture decision under OW-WAR-0074 AM-001. The path
identifies this reviewable draft; no enterprise or owner-reserved ADR identifier
is allocated. Human acceptance must name this exact decision and source subject.

## Decision

Add an opt-in source-set proposal to the existing SAS command:

```text
war sas propose <edition> --source <main-markdown> --source-set <manifest> --adr <decision>
war sas accept <edition>
war sign <edition> --ssh-sign
war sas status
```

The three source options are used together. Without them, proposal retains its
current single-document meaning. Repository-relative paths name the main SAS,
the candidate manifest, and this adoption decision. Revision records remain in
the existing configured revisions directory; candidate selection does not move
that directory or change `paths.sas`.

Source-set proposals use a new `oh.war/sas-revision/v2` record and versioned
acceptance request/response. Legacy v1 records and their serialized meaning remain
unchanged. The main SAS digest, manifest digest, decision digest, and complete
acceptance-subject digest are distinct fields and are displayed as such.

The complete subject is RFC 8785 canonical JSON with these meanings: schema
`oh.war/sas-source-set-subject/v1`, edition, repository-relative main-source path
and SHA-256, manifest path and SHA-256, adoption-decision path and SHA-256, and
the explicit predecessor revision/version and subject digest when present.
Its identity is SHA-256 of the UTF-8 prefix
`oh.war/sas-source-set-subject/v1` followed by one NUL byte and those canonical
JSON bytes. This new domain does not reinterpret any existing digest.

The manifest's exact bytes bind every member's path, length, digest and role.
All listed members are captured and checked, including reference fixtures. The
main source must be a normative member and the manifest edition must equal the
proposed edition. Section 106 is extracted from the same captured main bytes.
The predecessor's retained requirement IDs remain subject to existing stability
checks. Architecture-changing adoption requires the captured decision.

Capture retains manifest, member, and adoption-decision bytes under the configured
revision store in a per-edition snapshot. Publish a proposed record only after
the complete capture validates. Reject existing proposal/snapshot destinations
rather than overwriting them. Failed preparation produces no accepted record.
Do not follow symlinks, absolute paths, traversal segments, duplicate member
paths, or mismatched lengths/digests. Bound input sizes and reject inconsistent
reads. This is local capture, not a repository sandbox or online resolver.

Before acceptance, validate the proposed subject and snapshot and require the
declared current inputs to match that capture. Changed input requires a fresh
proposal. The authenticated response covers the whole subject, not only the main
SAS digest. Human role checks, refusal of agent acceptance, response signature
verification and separate attestation remain in force.

Accepted history is judged against retained captured bytes. A later live-source
edit cannot rewrite or invalidate what was historically accepted; status must
distinguish that historical integrity from whether current inputs match it.
Warrant compilation binds the complete v2 subject through its SAS pin. Old
Warrants retain their prior SAS pins until an authorized amendment changes them.
Historical literal `1.0.0` remains unchanged; it is designated RC.1 in edition
history. This work proposes `1.0.0-rc.2`; Stable acceptance and release naming
remain separate later acts.

## Required proof

Exercise proposal, read-only preview, acceptance, and status at their public
interfaces. Pair valid captures with changed main, companion, schema, reference,
manifest, decision and predecessor identities; missing/duplicate/escaping paths;
symlinks; oversized or inconsistent reads; stale or wrong-actor responses; and
existing-destination collisions. Verify no accepted record on refusal and
unchanged legacy record/signature behavior. Fixtures use fictional identities
in disposable repositories and never sign or accept the real candidate.

## Boundaries

This is acceptance bootstrap support. It does not implement the Phase 1 document
parser, general context compiler, batch signing, TUI, agent orchestration,
automatic acceptance, paid-call metering, or a new release. Existing resolved
delivery corrections and human signatures remain explicit requirements.
