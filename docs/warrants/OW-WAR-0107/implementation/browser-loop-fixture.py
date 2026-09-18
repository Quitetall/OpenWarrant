import json, os, uuid, hashlib
from pathlib import Path
from test_verifier_repair_execution import RepairExecutionTests
from server import Server
from hotline import Answers
from verifier_receipts import ReceiptInbox
from verifier_scheduler import Scheduler

t=RepairExecutionTests();t.setUp()
failed=t.verify(t.original['attempt_id'])
answers=Answers(t.executor,{'schema':'oh.war/hotline-config/v1','responders':[{'id':'fixture-human','kind':'human','governing_warrants':[t.original['warrant_id']],'token_sha256':hashlib.sha256(b'fixture-human-credential-for-ui-0001').hexdigest()}]})
t.service.answers=answers
s=Server(('127.0.0.1',0),t.store,'fixture-token');s.executor=t.executor;s.verification=t.service;s.hotline=answers;s.completion_word='WORK_DONE';s.report_detail='full'
meta={'url':f'http://127.0.0.1:{s.server_port}','pid':os.getpid(),'root':str(t.fixture.root),'key':str(t.key),'attempt_id':t.original['attempt_id'],'verification_id':failed['verification_id']}
Path('/tmp/ow107-loop-browser-meta.json').write_text(json.dumps(meta))
inbox=t.fixture.root/'receipt-inbox';inbox.mkdir()
s.verification_loops=Scheduler(t.service,ReceiptInbox(inbox,t.executor.decode),interval=.1)
s.verification_loops.start()
print(meta['url'],flush=True)
try:s.serve_forever()
finally:s.verification_loops.close();s.verification_loops.receipts.close();s.server_close();t.doCleanups()
