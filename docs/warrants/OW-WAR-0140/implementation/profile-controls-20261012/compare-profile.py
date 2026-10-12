import json,pathlib,hashlib,subprocess
s=pathlib.Path(__file__).resolve().parent
r=s/'profile-corpus'
def run(argv, dest):
 p=subprocess.run(argv,cwd=r,capture_output=True,timeout=180)
 (s/dest).write_bytes(p.stdout)
 if p.returncode: raise RuntimeError((argv,p.returncode,p.stderr.decode()[:1000]))
 return json.loads(p.stdout) if argv[1]=='status' else None
def matrix():
 return {p.parent.parent.name:{'ir_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'integrity':json.loads(p.read_text()).get('integrity'),'manifest_digest':hashlib.sha256((p.parent.parent/'manifest.toml').read_bytes()).hexdigest()} for p in sorted((r/'docs/warrants').glob('*/generated/WAR.json'))}
records={}
for label in ['before','after']:
 subprocess.run(['git','reset','--hard','c57215b1c371fe7b49e0fe54960638622b41a752'],cwd=r,check=True,capture_output=True)
 binary=s/('profile-'+label)/'target/debug/war'
 identity=subprocess.check_output([str(binary),'--version'],text=True,timeout=15).strip()
 run([str(binary),'compile','--json'],'corpus-'+label+'-compile.json')
 status=run([str(binary),'status','--json'],'corpus-'+label+'-status.json')
 m=matrix(); (s/('corpus-'+label+'-digests.json')).write_text(json.dumps(m,indent=2)+'\n')
 contracts={x['alias']:x['contract_digest'] for x in status['result']['warrants']}
 assert len(m)==142 and set(m)==set(contracts) and all(contracts.values())
 records[label]={'identity':identity,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'matrix':m,'contracts':contracts}
a,b=records['before'],records['after']
changed=[k for k in a['matrix'] if a['matrix'][k]!=b['matrix'].get(k)]
contract_changes=[k for k in a['contracts'] if a['contracts'][k]!=b['contracts'].get(k)]
obs={'schema':'openwarrant.historical-profile-observation/v1','before_commit':'975950a141aa038064e2e09d9ef19faef116268d','after_commit':'e995a450b12d2664dd72de2b6a51f2afddf7da2c','fixture_commit':'c57215b1c371fe7b49e0fe54960638622b41a752','same_fixture_path':True,'core_profile_definitions':'exact after-commit definitions added to otherwise before-commit corpus','warrants':142,'changed_ir_or_composition_digests':changed,'changed_contract_digests':contract_changes,'build_namespace':'automatic per-workspace build-dir','before_identity':a['identity'],'after_identity':b['identity'],'before_binary_sha256':a['binary_sha256'],'after_binary_sha256':b['binary_sha256'],'human_acceptance':False,'independent_verdict':False}
(s/'profile-comparison-observation.json').write_text(json.dumps(obs,indent=2)+'\n')
assert not changed and not contract_changes
print('142 unchanged complete IRs, composition and contract digests')
