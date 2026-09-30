# SPDX-License-Identifier: Apache-2.0
# Captured fixture procedure; local paths identify this evidence-producing checkout.
import pathlib, subprocess, tempfile, json, hashlib, shutil
source=pathlib.Path('/mnt/4tb/tmp/ow-export-preservation')
war=pathlib.Path('/mnt/4tb/tmp/ow107-gate-lineage/target/debug/war')
root=pathlib.Path(tempfile.mkdtemp(prefix='ow111-runtime-roundtrip-'))
repo=root/'source';repo.mkdir()
def call(args,cwd=repo):
 p=subprocess.run(args,cwd=cwd,capture_output=True)
 if p.returncode: raise RuntimeError((args,p.returncode,p.stdout.decode(),p.stderr.decode()))
 return p.stdout
call([str(war),'init','--namespace','ROUND','--program','Runtime roundtrip proof'])
call([str(war),'new','Retain service runtime after source removal'])
shutil.copytree(source/'schemas',repo/'schemas',dirs_exist_ok=True)
stage=repo/'docs/warrants/ROUND-WAR-0001/atoms/45-milestones.yaml'
stage.write_text(stage.read_text().replace('executor_kind: "agent"','executor_kind: "service"\n    executor_ref: "gate://ops.echo@1.0.0"\n    wall_time_seconds: 5'))
gate=repo/'docs/gates/ops.echo@1.0.0.yaml'
gate.write_text((source/'docs/gates/ops.echo@1.0.0.yaml').read_text().replace('argv: ["true"]','argv: ["sh", "-c", "printf retained-runtime"]'))
call(['git','init','-q']);call(['git','add','.']);call(['git','-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','source fixture'])
for _ in range(2):call([str(war),'run','ROUND-WAR-0001','STAGE-001'])
archive=root/'runtime.json'
call([str(war),'archive','export','ROUND-WAR-0001',str(archive),'--history'])
data=archive.read_bytes();value=json.loads(data)
shutil.rmtree(repo)
call([str(war),'archive','import',str(archive),str(root/'imported')],root)
call([str(war),'archive','reexport',str(root/'imported'),str(root/'again.json')],root)
assert data==(root/'again.json').read_bytes()
report={'fixture_only':True,'producer_source':call(['git','rev-parse','HEAD'],source).decode().strip(),'binary_sha256':hashlib.sha256(war.read_bytes()).hexdigest(),'source_removed_before_import':not repo.exists(),'byte_identical_reexport':True,'archive_sha256':hashlib.sha256(data).hexdigest(),'coverage':value.get('coverage'),'qualified':False,'authority_activated':False}
(root/'observation.json').write_text(json.dumps(report,indent=2)+'\n')
print(root)
