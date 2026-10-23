# Selection observations

`run.py --provider-root <LAMU-Rust-workspace> --output <new-file>` checks the
shared profile and runs the real provider's six selection tests. It retains
commands, output, input hashes, SDK pin and provider revision. It refuses to
replace an existing observation.

`check_observations.py --war <SDK-CLI> --producer <compile_package-example>
--compiler-build <observed-build-name> --record-dir <new-directory>` adds fresh
input checks through the real compiler example and a separate SDK consumer.
It records complete input documents and request/output/exit traces. The producer
receives random input directory names, not case labels. The seven cases cover
required and optional UNKNOWN pointers, known FALSE, changed source text,
cycles, missing required content and selected conflicts. Successful packages
are checked for exact selected-unit ranges, unchanged background text,
unselected entry content and repeatable output.

The example captures one document per call. Cross-source routing, access and
resource controls remain covered by the separate native tests. Original source
blobs remain in each package for integrity checks; selection limits the entry
and selected references, not access to individual units inside that audit blob.
Neither driver establishes trusted access, human acceptance or qualification.
