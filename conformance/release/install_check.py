#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Verify and exercise a candidate in a fresh temporary directory. No global install."""
import argparse
import hashlib
import gzip
import re
import sys
import io
import json
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import tempfile

LIMIT=256*1024*1024


def unique(pairs):
    result={}
    for k,v in pairs:
        if k in result:raise ValueError('Duplicate JSON field')
        result[k]=v
    return result


def verify(path,expected):
    if path.stat().st_size>LIMIT:raise ValueError('Archive size limit')
    data=path.read_bytes()
    if hashlib.sha256(data).hexdigest()!=expected:raise ValueError('Archive checksum mismatch')
    files={};total=0
    with gzip.GzipFile(fileobj=io.BytesIO(data)) as compressed:
        expanded=compressed.read(LIMIT+1)
    if len(expanded)>LIMIT:raise ValueError('Expanded archive size limit')
    with tarfile.open(fileobj=io.BytesIO(expanded),mode='r|') as archive:
        for member in archive:
            name=member.name
            if (len(files)>=8192 or not member.isfile() or name in files or not name
                or name.startswith('/') or '\\' in name or ':' in name
                or any(part in ('','.','..') for part in name.split('/'))
                or any(ord(c)<32 for c in name)):
                raise ValueError('Unsafe, duplicate or unsupported archive entry')
            total+=member.size
            if member.size<0 or total>LIMIT:raise ValueError('Expanded archive size limit')
            stream=archive.extractfile(member)
            if stream is None:raise ValueError('Missing entry bytes')
            value=stream.read(member.size+1)
            if len(value)!=member.size:raise ValueError('Entry length mismatch')
            files[name]=value
    if 'MANIFEST.json' not in files:raise ValueError('Missing manifest')
    manifest=json.loads(files['MANIFEST.json'],object_pairs_hook=unique)
    if manifest.get('schema')!='oh.war/candidate-bundle/v1' or manifest.get('qualified') is not False or manifest.get('promotable') is not False:
        raise ValueError('Unsupported candidate or false qualification')
    inventory=manifest['files']
    if set(inventory)!=set(files)-{'MANIFEST.json'}:raise ValueError('Manifest inventory mismatch')
    for name,item in inventory.items():
        if item['sha256']!=hashlib.sha256(files[name]).hexdigest() or item['bytes']!=len(files[name]):
            raise ValueError('Payload digest mismatch')
        if item['executable'] is not (name=='bin/war'):raise ValueError('Unexpected executable entry')
    required={'bin/war','sdk-source.tar','LICENSE','CANDIDATE.md'}
    if not required.issubset(files):raise ValueError('Required package component missing')
    return files,manifest


def extract_source(data,destination):
    if len(data)>LIMIT:raise ValueError('SDK source size limit')
    with tarfile.open(fileobj=io.BytesIO(data),mode='r:') as source:
        members=[];names=set();logical_bytes=0
        for member in source:
            path=member.name.rstrip('/')
            logical_bytes+=member.size
            if member.sparse is not None or member.size<0 or logical_bytes>LIMIT:
                raise ValueError('Sparse or over-limit SDK source member')
            if (len(members)>=16384 or path in names or not path or path.startswith('/')
                or any(p in ('','.','..') for p in path.split('/')) or re.match(r'^[A-Za-z]:',path) or '\\' in path
                or not (member.isfile() or member.isdir() or member.issym())):
                raise ValueError('Unsupported SDK source member')
            names.add(path);members.append(member)
        required={'crates/openwarrant-core/Cargo.toml','conformance/sdk/document/minimal-rc3.md',
                  '.claude/skills/openwarrant/LICENSE.mattpocock','.claude/skills/openwarrant/references/sdk-artifacts.md',
                  'docs/sas/drafts/1.0.0-rc.3/format-contract.md','docs/sas/drafts/1.0.0-rc.2/format-contract.md',
                  'conformance/sdk/cli/README.md','conformance/skills/check_skills.py'}
        if not required.issubset(names):raise ValueError('Required canonical source components missing')
        source.extractall(destination,members=members,filter='data')
    # These exact dependencies caught the original relocation defect. The complete
    # canonical workspace also retains the remaining source context.
    pairs=[('.claude/skills/openwarrant/references/sdk-artifacts.md','../../../../conformance/sdk/cli/README.md'),
           ('docs/sas/drafts/1.0.0-rc.3/format-contract.md','../1.0.0-rc.2/format-contract.md'),
           ('docs/sas/drafts/1.0.0-rc.3/sdk-contract.md','../../../../conformance/sdk/cli/README.md')]
    for page,target in pairs:
        file=destination/page
        if target not in file.read_text() or not (file.parent/target).resolve().is_relative_to(destination.resolve()) or not (file.parent/target).is_file():
            raise ValueError('Required installed context link missing')


def exercise(files,manifest):
    observations={}
    with tempfile.TemporaryDirectory(prefix='ow-install-') as tmp:
        root=Path(tmp)
        for name,data in files.items():
            path=root.joinpath(*PurePosixPath(name).parts);path.parent.mkdir(parents=True,exist_ok=True)
            with path.open('xb') as f:f.write(data)
            path.chmod(0o755 if name=='bin/war' else 0o644)
        source_root=root/'source';source_root.mkdir()
        extract_source(files['sdk-source.tar'],source_root)
        skill_check=subprocess.run([sys.executable,str(source_root/'conformance/skills/check_skills.py')],cwd=source_root,text=True,capture_output=True,timeout=30)
        observations['installed-skill-links']={'exit_code':skill_check.returncode,'stdout':skill_check.stdout,'stderr':skill_check.stderr}
        if skill_check.returncode:raise ValueError('Installed skill structure check failed')
        war=root/'bin/war'
        def run(name,args,input=None):
            result=subprocess.run([str(war),*args],cwd=root,input=input,text=True,capture_output=True,timeout=30)
            observations[name]={'argv':args,'exit_code':result.returncode,'stdout':result.stdout,'stderr':result.stderr}
            if result.returncode:raise ValueError(name+' failed')
            return result.stdout
        if run('version',['--version']).strip()!=manifest['binary_version']:raise ValueError('Installed version mismatch')
        source=(source_root/'conformance/sdk/document/minimal-rc3.md').read_text()
        request={'schema':'oh.war/sdk-request/v1','operation':'validate','dialect':'rc3','source':source}
        result=json.loads(run('sdk-example',['sdk','--request','-'],json.dumps(request)))
        if result['result']['validity']!='Valid' or result['result']['qualification_established']:raise ValueError('SDK example standing mismatch')
        result=json.loads(run('interactive',['--json','document','draft','--draft-dir','draft','--output','example.md'],'save\n'))
        if not result['result']['saved'] or result['result']['qualified']:raise ValueError('Authoring walkthrough failed')
        destination=str(root)
    if Path(destination).exists():raise ValueError('Temporary installation not removed')
    return observations


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--archive',required=True,type=Path)
    p.add_argument('--sha256',required=True);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    if a.output.exists():raise ValueError('Receipt destination exists')
    files,manifest=verify(a.archive,a.sha256);observations=exercise(files,manifest)
    receipt={'schema':'oh.war/install-observation/v1','passed':True,'archive_sha256':a.sha256,'manifest':manifest,
             'observations':observations,'temporary_install_removed':True,'qualification_established':False}
    with a.output.open('x') as f:json.dump(receipt,f,indent=2);f.write('\n')
    print(json.dumps({'passed':True,'receipt':str(a.output),'qualification_established':False}))
