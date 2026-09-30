#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Run the fixed interactive transcript against an explicit built executable."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--war',required=True,type=Path)
p.add_argument('--output',required=True,type=Path)
a=p.parse_args()
if a.output.exists():raise ValueError('Receipt destination exists')
war=a.war.resolve(strict=True)
transcript=Path(__file__).with_name('author.stdin').read_bytes()
binary=hashlib.sha256(war.read_bytes()).hexdigest()
observations={}
with tempfile.TemporaryDirectory(prefix='ow88-interactive-') as temp:
    root=Path(temp)
    def run(name,args,data):
        process=subprocess.run([str(war),*args],input=data,cwd=root,capture_output=True,timeout=30)
        observations[name]={'argv':args,'input_sha256':hashlib.sha256(data).hexdigest(),
                            'exit_code':process.returncode,'stdout':process.stdout.decode(),'stderr':process.stderr.decode()}
        return process.returncode,json.loads(process.stdout)
    base=['--json','document','draft','--draft-dir','draft','--output','result.md']
    code,result=run('author',base,transcript)
    if code or not result['result']['saved'] or result['result']['qualified']:raise ValueError('Author failed')
    source=(root/'result.md').read_bytes()
    state=json.loads(sorted((root/'draft').glob('draft-*.json'))[-1].read_bytes())
    request={'schema':'oh.war/sdk-request/v1','operation':'author','metadata':state['metadata'],'units':state['units']}
    code,result=run('sdk-parity',['sdk','--request','-'],json.dumps(request).encode())
    if code or result['result']['source'].encode()!=source:raise ValueError('SDK parity failed')
    code,result=run('cancel',[*base,'--resume'],b'cancel\n')
    if code or result['result']['saved']:raise ValueError('Cancel published')
    code,result=run('existing-output',[*base,'--resume'],b'save\n')
    if code==0 or (root/'result.md').read_bytes()!=source:raise ValueError('Existing output changed')
    alternate=[*base];alternate[-1]='resumed.md'
    code,result=run('resume',[*alternate,'--resume'],b'save\n')
    if code or (root/'resumed.md').read_bytes()!=source:raise ValueError('Resume changed source')
if hashlib.sha256(war.read_bytes()).hexdigest()!=binary:raise ValueError('Executable changed during run')
receipt={'schema':'oh.war/interactive-observation/v1','passed':True,'war':str(war),'binary_sha256':binary,
         'transcript_sha256':hashlib.sha256(transcript).hexdigest(),'observations':observations,
         'qualification_established':False,'human_effort_measured':False}
with a.output.open('x') as f:json.dump(receipt,f,indent=2);f.write('\n')
print(json.dumps({'passed':True,'receipt':str(a.output)}))
