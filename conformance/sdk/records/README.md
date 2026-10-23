# Supplied records and workflow facts

The SDK evaluates explicit inputs. It does not authenticate a caller's facts,
run a workflow, dispatch an agent or sign a result. A conditional `eligible`
assessment is not an assurance mark.

Run the public library controls and file-backed codec cases:

```sh
cargo test --locked -p openwarrant-core --test document_records --test document_workflow_records --test document_schedule
cargo run --locked -p openwarrant-core --example sdk_probe -- --scope 83 --fixtures conformance/sdk
cargo test --locked -p openwarrant-cli --test all sdk_cli::
```

To retain the actual CLI requests and responses outside the protected fixtures:

```sh
python3 conformance/sdk/records/check_observations.py \
  --war /absolute/path/to/war \
  --cases conformance/sdk/cli/cases.json \
  --record-dir /absolute/path/to/new-observation-directory
```

The observer executes the record, workflow and scheduling cases with their
existing expected fields and refusal codes. It also generates new scheduling
subjects and checks exact contract/stage matching, ambiguous output refusal,
explicit output selection and action gates versus qualification conditions.
It sends no case labels to the implementation and contains no second evaluator.

The output directory must be new. `sdk-wire.jsonl` retains exact input, output,
stderr and exit status for each call, including a failing call. Only a fully
passing observation publishes `observation.json`. Failed observations remain
failed; use a different directory after a repair. Fixture bytes are checked for
changes. Binary and fixture hashes identify the observation, not a human review.

The library tests additionally cover limits, independence, exact human acceptance
facts, evidence loss, batch review, stops, resume and handoff. Their synthetic
trust facts are test inputs. They are not real signatures or independent verdicts
for the implementation being tested.
