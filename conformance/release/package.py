#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Build a local candidate bundle. This tool never publishes or qualifies a release."""
import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import tarfile
import tempfile

ROOT=Path(__file__).resolve().parents[2]
LIMIT=256*1024*1024


def sha(data):return hashlib.sha256(data).hexdigest()


def git(*args):return subprocess.check_output(['git',*args],cwd=ROOT)


def committed(path,head):return git('show',head+':'+path)


def source_inventory(head):
    entries={};total=0
    for row in git('ls-tree','-r','-z','-l',head).split(b'\0'):
        if not row:continue
        info,name=row.split(b'\t',1);mode,kind,oid,size=info.split()
        if kind!=b'blob':raise ValueError('Submodule source requires an explicit release profile')
        total+=int(size)
        if total>LIMIT:raise ValueError('Source archive input limit exceeded')
        entries[name.decode()]={'mode':mode.decode(),'object':oid.decode(),'bytes':int(size)}
    if not entries:raise ValueError('Empty source inventory')
    return entries


def publish_new(path,data):
    # Same-directory staging, sync, and hard link: an existing path is never replaced.
    fd,name=tempfile.mkstemp(prefix='.ow-package-',dir=path.parent)
    try:
        with os.fdopen(fd,'wb') as f:f.write(data);f.flush();os.fsync(f.fileno())
        os.link(name,path)
    finally:os.unlink(name)


def license_notices(head):
    host=subprocess.check_output(['rustc','-vV'],cwd=ROOT,text=True).split('host: ',1)[1].splitlines()[0]
    metadata=json.loads(subprocess.check_output(['cargo','metadata','--offline','--locked','--format-version','1','--filter-platform',host],cwd=ROOT))
    files={};inventory=[]
    overrides=json.loads((Path(__file__).parent/'licenses/provenance.json').read_text())
    for path in git('ls-files').decode().splitlines():
        if path.endswith(('Cargo.toml','Cargo.lock')) and (ROOT/path).read_bytes()!=committed(path,head):
            raise ValueError('Cargo source differs from candidate commit')
    for package in metadata['packages']:
        root=Path(package['manifest_path']).parent.resolve()
        paths=[p for p in root.iterdir() if p.is_file() and p.name.upper().startswith(('LICENSE','COPYING','NOTICE'))]
        if package.get('license_file'):
            path=(root/package['license_file']).resolve()
            if not path.is_relative_to(root):raise ValueError('License path escapes package')
            paths.append(path)
        notices=[]
        for path in sorted(set(paths)):
            if path.stat().st_size>1024*1024:raise ValueError('License file limit exceeded')
            name='licenses/'+package['name']+'-'+package['version']+'/'+path.name
            if name in files:raise ValueError('Duplicate license file identity')
            files[name]=path.read_bytes();notices.append(name)
        if not notices and package.get('source') is None and root.is_relative_to(ROOT):
            name='licenses/'+package['name']+'-'+package['version']+'/LICENSE'
            files[name]=committed('LICENSE',head);notices.append(name)
        override=overrides.get(package['name']+'@'+package['version'])
        if not notices and override:
            vcs=json.loads((root/'.cargo_vcs_info.json').read_text())
            data=(Path(__file__).parent/'licenses'/override['file']).read_bytes()
            if vcs['git']['sha1']!=override['source_commit'] or sha(data)!=override['sha256']:
                raise ValueError('Upstream license provenance mismatch')
            name='licenses/'+package['name']+'-'+package['version']+'/LICENSE'
            files[name]=data;notices.append(name)
        if not notices:raise ValueError('Missing license text: '+package['name']+' '+package['version'])
        inventory.append({'name':package['name'],'version':package['version'],'source':package['source'],
                          'license':package['license'],'notices':notices,'upstream_notice':override})
    files['THIRD_PARTY.json']=(json.dumps({'schema':'oh.war/license-inventory/v1','target':host,'packages':inventory},indent=2)+'\n').encode()
    return files


def bundle(war,output,label):
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,95}',label):raise ValueError('Unsafe candidate label')
    if output.exists() or output.is_symlink():raise ValueError('Output directory already exists')
    if platform.system() not in ['Linux','Darwin']:raise ValueError('Unsupported host profile')
    if war.stat().st_size>LIMIT:raise ValueError('Binary input limit exceeded')
    binary=war.read_bytes();head=git('rev-parse','HEAD').decode().strip();inventory=source_inventory(head)
    version=subprocess.run([str(war),'--version'],capture_output=True,text=True,timeout=30)
    if version.returncode or not version.stdout.startswith('war '):raise ValueError('Supplied binary did not identify as war')
    files={'bin/war':binary,'LICENSE':committed('LICENSE',head)}
    files.update(license_notices(head))
    required=['.claude/skills/openwarrant/LICENSE.mattpocock','docs/sas/drafts/1.0.0-rc.3/source-set.json',
              'docs/sas/drafts/1.0.0-rc.2/format-contract.md','conformance/sdk/cli/README.md']
    if any(name not in inventory for name in required):raise ValueError('Required source component missing')
    # Full committed workspace retains manifests, SDK source and all fixture paths.
    source=git('archive','--format=tar',head)
    if len(source)>LIMIT:raise ValueError('Source archive output limit exceeded')
    files['sdk-source.tar']=source
    files['CANDIDATE.md']=('''# OpenWarrant candidate rehearsal\n\nThis bundle is unverified. It does not establish Stable 1.0 or permission to deploy.\nExtract sdk-source.tar into a separate source/ directory. This preserves all\ncanonical paths: standard at source/docs/sas/drafts/1.0.0-rc.3/, SDK at\nsource/crates/openwarrant-core/, and adapted skills at source/.claude/skills/.\nTheir relative links and original license notices remain intact. Do not relocate\nindividual skill or standard files away from their referenced source context.\nOpenWarrant owns the document standard and SDK. LAMU owns semantic compilation;\nworkflow applications own orchestration, UI, storage and execution policy.\nThe CLI includes historical document compilation, not a replacement semantic compiler.\n\nVerify MANIFEST.json against every unpacked file before running bin/war.\nUse conformance/release/install_check.py from a trusted checkout to inspect a bundle.\nKeep the unpacked directory separate: removing it rolls back this local installation;\nno shell profile, global binary or existing repository is modified.\nThe supplied binary is exercised but its linkage to this source commit is not\ncryptographically authenticated. Final qualification needs native build provenance.\n''').encode()
    if sum(map(len,files.values()))>LIMIT:raise ValueError('Bundle payload limit exceeded')
    manifest={'schema':'oh.war/candidate-bundle/v1','label':label,'source_git_head':head,
              'host':platform.system(),'architecture':platform.machine(),'binary_version':version.stdout.strip(),
              'source_components':{'sdk':'crates/openwarrant-core','standard':'docs/sas/drafts/1.0.0-rc.3','skills':'.claude/skills'},
              'binary_source_linkage':'caller-supplied-unestablished','qualified':False,'promotable':False,
              'packager_sha256':sha(Path(__file__).read_bytes()),
              'files':{name:{'sha256':sha(data),'bytes':len(data),'executable':name=='bin/war'} for name,data in sorted(files.items())}}
    files['MANIFEST.json']=(json.dumps(manifest,sort_keys=True,indent=2)+'\n').encode()
    buffer=io.BytesIO()
    with gzip.GzipFile(fileobj=buffer,mode='wb',mtime=0,filename='') as gz:
        with tarfile.open(fileobj=gz,mode='w') as archive:
            for name,data in sorted(files.items()):
                member=tarfile.TarInfo(name);member.size=len(data);member.mode=0o755 if name=='bin/war' else 0o644
                member.mtime=0;archive.addfile(member,io.BytesIO(data))
    if git('rev-parse','HEAD').decode().strip()!=head or sha(war.read_bytes())!=sha(binary):
        raise ValueError('Source or executable changed during packaging')
    output.mkdir()
    name=f'openwarrant-{label}-{platform.system().lower()}-{platform.machine()}.tar.gz'
    data=buffer.getvalue();publish_new(output/name,data)
    publish_new(output/(name+'.sha256'),(sha(data)+'  '+name+'\n').encode())
    publish_new(output/'candidate.json',(json.dumps({'archive':name,'sha256':sha(data),'manifest':manifest},indent=2)+'\n').encode())
    return output/name


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--war',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True);p.add_argument('--label',default='1.0.0-rc.3-rehearsal')
    a=p.parse_args()
    print(json.dumps({'archive':str(bundle(a.war.resolve(strict=True),a.output,a.label)),'qualified':False,'promotable':False}))
