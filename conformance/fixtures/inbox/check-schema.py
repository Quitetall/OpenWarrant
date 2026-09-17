#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Validate real CLI output against proposed ADR0018; requires jsonschema."""
import copy
import json
from pathlib import Path
import subprocess
import sys

from jsonschema import Draft202012Validator, FormatChecker

if 'date-time' not in FormatChecker.checkers:
    sys.exit('Missing date-time validator; install jsonschema[format] before running this check')

here = Path(__file__).resolve().parent
binary = Path(sys.argv[1]).resolve()
schema = json.loads((here / 'inbox.schema.json').read_text())
Draft202012Validator.check_schema(schema)
validator = Draft202012Validator(schema, format_checker=FormatChecker())
output = subprocess.check_output([str(binary), 'inbox', '--json'], cwd=here / 'repository')
result = json.loads(output)['result']
validator.validate(result)
validator.validate(json.loads(json.dumps(result)))
refused = 0
for field in schema['required']:
    damaged = copy.deepcopy(result)
    del damaged[field]
    assert list(validator.iter_errors(damaged)), field
    refused += 1
for field in schema['properties']['items']['items']['required']:
    damaged = copy.deepcopy(result)
    del damaged['items'][0][field]
    assert list(validator.iter_errors(damaged)), field
    refused += 1
for field, value in [('awaited_act', 'execute'), ('state', 'invented'), ('waiting_since', 'yesterday')]:
    damaged = copy.deepcopy(result)
    damaged['items'][0][field] = value
    assert list(validator.iter_errors(damaged)), field
    refused += 1
print(f'PASS: CLI result and JSON roundtrip; {refused} malformed schema controls refused')
