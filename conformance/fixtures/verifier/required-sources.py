# SPDX-License-Identifier: Apache-2.0
# Synthetic disposable CLI control; no production verdict or model call.
from pathlib import Path
import shutil,subprocess,json,hashlib,sys
repo=Path(sys.argv[1])
binary=sys.argv[2]
root=Path(sys.argv[3])/'repository'
shutil.copytree(repo/'conformance/fixtures/inbox/repository',root,dirs_exist_ok=True)
gates=root/'docs/gates';gates.mkdir(parents=True,exist_ok=True)
gate=(repo/'docs/gates/software.repo.war-check@1.0.0.yaml').read_text()+'\nfixtures: ["fixtures/required.bin"]\n'
(gates/'software.repo.war-check@1.0.0.yaml').write_text(gate)
(root/'fixtures').mkdir();(root/'fixtures/required.bin').write_bytes(bytes([0,255,13,10]))
def run(args):
 p=subprocess.run([binary,*args,'--json'],cwd=root,capture_output=True,text=True)
 return json.loads(p.stdout)
report=run(['verify','IX-WAR-0003','--performer','fixture-performer','--bundle'])
assert report['exit_code']==0,report
ref=report['result']['packets'][0];original_path=root/ref['path'];packet=json.loads(original_path.read_text())
assert any(s['path']=='fixtures/required.bin' for s in packet['required_sources'])
def digest(payload):
 def check(value):
  assert not isinstance(value,float)
  if isinstance(value,dict):
   assert all(k.isascii() for k in value)
   for v in value.values():check(v)
  elif isinstance(value,list):
   for v in value:check(v)
 check(payload)
 data=json.dumps({'digest_domain':'oh.war/verification-bundle/v1','payload':payload},ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()
 return hashlib.sha256(data).hexdigest()
assert digest(packet)==ref['digest'],'probe must reproduce existing canonical digest before mutation'
original=json.loads(json.dumps(packet))
packet['required_sources']=[s for s in packet['required_sources'] if s['path']!='fixtures/required.bin']
newdigest=digest(packet);newrelative='docs/warrants/IX-WAR-0003/verifications/bundle-'+newdigest[:16]+'.json'
(root/newrelative).write_text(json.dumps(packet,ensure_ascii=False))
request=packet['request']
flags={'performer_transcript_blind':True,'performer_rationale_blind':True,'separate_writable_workspace':True,'cannot_modify_subject_artifacts':True,'cannot_modify_gate_definition':True,'cannot_modify_gate_fixtures':True,'separate_context_compilation':True,'distinct_model_required':False,'distinct_human_required':False}
response={'schema':'oh.war/verification-response/v2','warrant':'IX-WAR-0003','reviewed_subject':request['reviewed_subject'],'reviewed_packets':[{'path':newrelative,'digest':newdigest}],'verifications':[{'obligation':o['id'],'disposition':'established','evidence':'synthetic disposable probe only; not production assurance','performer':'fixture-performer','verifier':{'actor':'fixture-independent-verifier','kind':'service','independence':flags}} for o in request['obligations']]}
def toml(v):
 if isinstance(v,dict):return '{'+', '.join(json.dumps(k)+' = '+toml(x) for k,x in v.items())+'}'
 if isinstance(v,list):return '['+', '.join(toml(x) for x in v)+']'
 return json.dumps(v,ensure_ascii=False)
response_path=root.with_suffix('.response.toml');response_path.write_text('\n'.join(json.dumps(k)+' = '+toml(v) for k,v in response.items())+'\n')
paths=[root/'docs/warrants/IX-WAR-0003'/p for p in ['verifications/OBL-001.toml','verifications/OBL-002.toml','journal.jsonl']]
def snapshot():return [p.read_bytes() if p.exists() else None for p in paths]
before=snapshot()
result=run(['verify','IX-WAR-0003','--response',str(response_path)])
assert any(d['rule']=='verify.packet-binding' for d in result['diagnostics']),result
assert snapshot()==before,'refusal changed verdicts or journal'
# A complete source set cannot justify a substituted question, either.
packet=json.loads(json.dumps(original))
packet['request']['obligations'][0]['statement']='a weaker substituted task'
newdigest=digest(packet)
newrelative='docs/warrants/IX-WAR-0003/verifications/bundle-'+newdigest[:16]+'.json'
(root/newrelative).write_text(json.dumps(packet,ensure_ascii=False))
response['reviewed_packets']=[{'path':newrelative,'digest':newdigest}]
response_path.write_text('\n'.join(json.dumps(k)+' = '+toml(v) for k,v in response.items())+'\n')
result=run(['verify','IX-WAR-0003','--response',str(response_path)])
assert any(d['rule']=='verify.packet-binding' for d in result['diagnostics']),result
assert snapshot()==before,'task refusal changed verdicts or journal'
response['reviewed_packets']=[ref]
response_path.write_text('\n'.join(json.dumps(k)+' = '+toml(v) for k,v in response.items())+'\n')
restored=run(['verify','IX-WAR-0003','--response',str(response_path)])
assert restored['exit_code']==0,restored
print('rehashed missing fixture and substituted task refused without writes; exact original packet accepted')
