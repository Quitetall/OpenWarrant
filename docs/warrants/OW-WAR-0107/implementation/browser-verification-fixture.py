import json,sys,subprocess,os
from pathlib import Path
from test_execution import ExecutionTests, WAR
from server import Store, SDK, Server, read_file, publish, decode
from execution import Executor
from verifier_service import Verification

t=ExecutionTests();t.setUp();draft=t.eligible()
code,started=t.call('/api/runs','POST',{'warrant_id':draft['id'],'source_sha256':draft['source_sha256']})
assert code==202,(code,started)
finished=t.wait_run(started['attempt_id']);assert finished['work_state']=='completed',finished
t.stop()
key=t.root/'fixture-machine-key'
subprocess.run(['ssh-keygen','-q','-t','ed25519','-N','','-f',str(key)],check=True)
public=' '.join(key.with_suffix('.pub').read_text().split()[:2])
program=t.root/'reviewer.py'
program.write_text("import json,sys,hashlib\nr=json.load(sys.stdin)\nh=hashlib.sha256(json.dumps(r,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()\nprint(json.dumps({'schema':'oh.war/verification-result/v1','verification_id':r['verification_id'],'request_sha256':h,'verdict':'pass','summary':'Fixture independently read candidate and passed required checks','findings':[]}))\n")
config=t.root/'verifier.json';issuer=t.root/'issuer.json'
config.write_text(json.dumps({'schema':'oh.war/verifier-config/v1','performer':{'id':'fixture-worker','argv':t.config['argv']},'verifier':{'id':'fixture-reviewer','argv':[sys.executable,str(program)]},'cost_mode':'free','spend_limit_usd':10,'timeout_seconds':30}))
issuer.write_text(json.dumps({'schema':'oh.war/verifier-issuer/v1','principal':'fixture','public_key':public}))
store=Store(t.root/'state',SDK(Path(WAR),t.repo))
s=Server(('127.0.0.1',0),store,'fixture-token');s.completion_word='WORK_DONE';s.report_detail='full'
s.executor=Executor(store,t.config_path,read_file,publish,decode)
s.verification=Verification(s.executor,config,issuer)
meta={'url':f'http://127.0.0.1:{s.server_port}','pid':os.getpid(),'root':str(t.root),'key':str(key),'attempt_id':started['attempt_id']}
Path('/tmp/ow107-browser-meta.json').write_text(json.dumps(meta))
print(meta['url'],flush=True)
try:s.serve_forever()
finally:s.server_close();os.close(store.lock_fd);t.tmp.cleanup()
