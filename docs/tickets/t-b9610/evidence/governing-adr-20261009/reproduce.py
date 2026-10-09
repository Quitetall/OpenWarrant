# SPDX-License-Identifier: Apache-2.0
# Synthetic disposable public-CLI fixture. No live verdict or authority act.
import json,os,shutil,subprocess,sys,tempfile,tomllib
from pathlib import Path
repo=Path(sys.argv[1]);binary=Path(sys.argv[2]);out=Path(sys.argv[3])
with tempfile.TemporaryDirectory(prefix='ow-governing-context-') as tmp:
 root=Path(tmp)/'repository';shutil.copytree(repo/'conformance/fixtures/inbox/repository',root)
 if '--minimal' in sys.argv:
  for path in (root/'docs/warrants').glob('IX-WAR-*'):
   if path.name!='IX-WAR-0003': shutil.rmtree(path)
 manifest=tomllib.loads((root/'docs/warrants/IX-WAR-0003/manifest.toml').read_text())
 rule=f'''---
schema: oh.war/atom/v1
adr_uuid: 01a0f502-4941-70a1-a446-e1eb77dff191
local_alias: IX-ADR-0001
role: adr
jurisdiction: bound
order: 30
classification: internal
status: accepted
governs:
  - "war://{manifest['uuid']}"
---

# Synthetic governing decision

Email verification is required before account activation.
'''
 path=root/'docs/adr/atoms/IX-ADR-0001.md';path.parent.mkdir(parents=True);path.write_text(rule)
 if '--pin' in sys.argv:
  (root/'docs/sas').mkdir(exist_ok=True)
  (root/'docs/sas/Inbox_SAS.md').write_text('# Inbox SAS\n\n## 106. Requirements\n\n| ID | Requirement |\n| --- | --- |\n| IX-SAS-RQ-001 | Email verification is required before account activation. |\n')
  pin=subprocess.run([str(binary),'sas','propose','0.1.0','--json'],cwd=root,env={**os.environ,'OPENWARRANT_NO_PROJECTS':'1','OPENWARRANT_NO_UPDATE_CHECK':'1'},capture_output=True)
  assert pin.returncode==0,pin.stdout
 args=[str(binary),'verify','IX-WAR-0003','--performer','synthetic-performer','--bundle','--json']
 proc=subprocess.run(args,cwd=root,env={**os.environ,'OPENWARRANT_NO_PROJECTS':'1','OPENWARRANT_NO_UPDATE_CHECK':'1'},capture_output=True,timeout=20)
 (out/'stdout.json').write_bytes(proc.stdout);(out/'stderr.txt').write_bytes(proc.stderr)
 response=json.loads(proc.stdout);assert response['exit_code']==0,response
 paths=[root/p['path'] for p in response['result']['packets']]
 packets=[json.loads(p.read_text()) for p in paths]
 required=[s['path'] for p in packets for s in p.get('required_sources',[])]
 print(json.dumps({'command':args,'exit':proc.returncode,'reviewed_context':response['result'].get('reviewed_subject',{}).get('context_sources',{}),'required_sources':required,'expected':'docs/adr/atoms/IX-ADR-0001.md','synthetic':True}))
 assert 'docs/adr/atoms/IX-ADR-0001.md' in required,'governing decision omitted from offline verifier context'
