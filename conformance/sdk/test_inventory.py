# SPDX-License-Identifier: Apache-2.0
import json
from pathlib import Path
import tempfile
import subprocess
import unittest
from run_phase1 import ROOT, inventory, source_inventory, source_observation, built_executable


class Inventory(unittest.TestCase):
    def test_complete_inventory_admitted(self):
        self.assertEqual(len(inventory(ROOT)['cases']),96)

    def test_missing_assignment_and_fixture_case_refuse(self):
        original=json.loads((ROOT/'conformance/sdk/phase1.json').read_text())
        for mutation in ['assignment','fixture']:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as tmp:
                root=Path(tmp)
                paths=['conformance/sdk/phase1.json','docs/sas/drafts/1.0.0-rc.3/phase-1-build-scope.md']
                paths+=list(original['required_fixture_ids'])+list(original['required_test_functions'])
                for name in paths:
                    path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes((ROOT/name).read_bytes())
                if mutation=='assignment':
                    value=json.loads((root/paths[0]).read_text());del value['cases']['SDK-07']
                    (root/paths[0]).write_text(json.dumps(value))
                else:
                    path=root/'conformance/sdk/document/cases.json'
                    value=json.loads(path.read_text());value['cases'].pop(0);path.write_text(json.dumps(value))
                with self.assertRaisesRegex(ValueError,'inventory.'):
                    inventory(root)

    def test_build_inputs_and_midrun_drift(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            subprocess.run(['git','init','-q',str(root)],check=True)
            for name in ['Cargo.toml','Cargo.lock','rust-toolchain.toml','crates/compiler/src/lib.rs']:
                path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text('original')
            before=source_inventory(root)
            self.assertEqual(len(before),4)
            self.assertEqual(source_observation(before,before)['state'],'PASS')
            (root/'Cargo.lock').write_text('different dependencies')
            after=source_inventory(root)
            self.assertEqual(source_observation(before,after),{'state':'FAIL','changed_paths':['Cargo.lock']})

    def test_executable_comes_from_cargo_artifact(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary=Path(tmp)/'custom-target'/'debug'/'war'
            binary.parent.mkdir(parents=True);binary.write_bytes(b'fixture executable identity')
            line=json.dumps({'reason':'compiler-artifact','target':{'name':'war'},'executable':str(binary)})
            self.assertEqual(built_executable(line),binary)
            with self.assertRaisesRegex(ValueError,'build.executable'):
                built_executable(json.dumps({'reason':'build-finished','success':True}))


if __name__=='__main__':unittest.main()
