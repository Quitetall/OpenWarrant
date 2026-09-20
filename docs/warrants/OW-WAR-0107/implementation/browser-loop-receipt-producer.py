import json,urllib.request,subprocess,time,base64,os
from pathlib import Path
m=json.load(open('/tmp/ow107-loop-browser-meta.json'))
def get(path):
 return json.load(urllib.request.urlopen(urllib.request.Request(m['url']+path,headers={'Authorization':'Bearer fixture-token'})))
issued=set(); deadline=time.monotonic()+10
while time.monotonic()<deadline:
 status=get('/api/verification-loops')['loops'][0]
 if (status.get('observation') or {}).get('state')=='checks_passed':
  print(json.dumps(status));break
 for j in get('/api/verification')['jobs']:
  if j['state']!='prepared' or j['verification_id'] in issued:continue
  now=int(time.time());p=json.dumps({'schema':'oh.war/harness-protection/v1','basis_sha256':j['basis_sha256'],'nonce':j['verification_id'],'issued_at_unix':now,'expires_at_unix':now+120,'evidence_ref':'fixture://browser-loop','protections':{k:'pass' for k in ['separate-context','read-only-candidate','protected-checks','protected-control-storage']}}).encode()
  sig=subprocess.run(['ssh-keygen','-Y','sign','-f',m['key'],'-n','openwarrant-harness-protection'],input=p,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True).stdout
  path=Path(m['root'])/'receipt-inbox'/(j['verification_id']+'.json');tmp=path.with_suffix('.pending')
  tmp.write_text(json.dumps({'payload_base64':base64.b64encode(p).decode(),'signature_base64':base64.b64encode(sig).decode()}));os.rename(tmp,path);issued.add(j['verification_id'])
 time.sleep(.05)
else: raise RuntimeError('Fixture loop did not settle')
print('Synthetic receipts:',len(issued))
