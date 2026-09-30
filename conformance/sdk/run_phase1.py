#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Run current-host Phase 1 acceptance; retain explicit platform/qualification gaps."""
import argparse
import hashlib
import json
import platform
from pathlib import Path
import re
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[2]


def inventory(root):
    value=json.loads((root/'conformance/sdk/phase1.json').read_text())
    expected=({f'S{n:02}' for n in range(1,9)} | {f'SDK-{n:02}' for n in range(1,25)} |
              {f'FOOTER-{n:02}' for n in range(1,9)} | {f'T{n:02}' for n in range(1,57)})
    if value.get('schema')!='oh.war/phase1-inventory/v1' or set(value['cases'])!=expected:
        raise ValueError('inventory.missing-or-extra-case')
    declared=set(re.findall(r'^- (SDK-\d{2}|FOOTER-\d{2}):',
                           (root/'docs/sas/drafts/1.0.0-rc.3/phase-1-build-scope.md').read_text(),re.M))
    if declared!={key for key in expected if key.startswith(('SDK-','FOOTER-'))}:
        raise ValueError('inventory.spec-drift')
    for key,case in value['cases'].items():
        provider=key.startswith('T') and 11<=int(key[1:])<=41
        if case['phase']!=(2 if provider else 1) or (not provider and not case['checks']):
            raise ValueError('inventory.phase-or-check')
    for path,expected_ids in value['required_fixture_ids'].items():
        actual=[c['id'] for c in json.loads((root/path).read_text())['cases']]
        if actual!=expected_ids or len(set(actual))!=len(actual):
            raise ValueError('inventory.fixture-case-drift: '+path)
    for path,names in value['required_test_functions'].items():
        actual=re.findall(r'#\[test\]\s*fn\s+(\w+)',(root/path).read_text())
        if actual!=names or not names:raise ValueError('inventory.test-drift: '+path)
    return value


def source_inventory(root, excluded=None):
    """Bind tracked and untracked build/corpus inputs without following symlinks."""
    names=subprocess.check_output(['git','ls-files','-z','--cached','--others','--exclude-standard'],cwd=root).decode().split('\0')
    files={}
    for name in sorted(set(names)-{''}):
        path=root/name
        if '__pycache__' in path.parts or path.suffix=='.pyc' or path==excluded:
            continue
        if path.is_symlink():
            import os
            data=b'symlink\0'+os.fsencode(os.readlink(path))
        elif path.is_file():data=path.read_bytes()
        else:continue
        files[name]=hashlib.sha256(data).hexdigest()
    return files


def source_observation(before, after):
    changed=sorted(k for k in before.keys()|after.keys() if before.get(k)!=after.get(k))
    return {'state':'FAIL' if changed else 'PASS','changed_paths':changed}


def built_executable(stdout):
    paths={item['executable'] for line in stdout.splitlines()
           if (item:=json.loads(line)).get('reason')=='compiler-artifact'
           and item.get('target',{}).get('name')=='war' and item.get('executable')}
    if len(paths)!=1:raise ValueError('build.executable-identity-unavailable')
    return Path(paths.pop()).resolve(strict=True)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output',required=True,type=Path)
    args=p.parse_args()
    if args.output.exists():raise ValueError('receipt destination exists')
    before=source_inventory(ROOT,args.output.resolve())
    value=inventory(ROOT)
    identity={}
    for name,argv in {'git_head':['git','rev-parse','HEAD'],'rustc':['rustc','-Vv'],'cargo':['cargo','-V']}.items():
        identity[name]=subprocess.check_output(argv,cwd=ROOT,text=True).strip()
    commands={
        'build':['cargo','build','--locked','-p','openwarrant-cli','--bin','war','--message-format=json'],
        'fixtures':['cargo','run','--locked','-p','openwarrant-core','--example','sdk_probe','--','--scope','85','--fixtures','conformance/sdk'],
        'core':['cargo','test','--locked','-p','openwarrant-core','--tests'],
        'cli':['cargo','test','--locked','-p','openwarrant-cli','--test','sdk_cli'],
        'skills':[sys.executable,'conformance/sdk/skills/check_artifacts.py','--war','<built executable>'],
        'context':[sys.executable,'-m','unittest','discover','-s','conformance/sdk/skills','-p','test_*.py'],
        'stage':['cargo','test','--locked','-p','openwarrant-core','--test','skill_artifacts'],
        'structure':[sys.executable,'conformance/skills/check_skills.py'],
        'inventory':[sys.executable,'-m','unittest','discover','-s','conformance/sdk','-p','test_inventory.py'],
    }
    if {check for case in value['cases'].values() for check in case['checks']}-commands.keys():
        raise ValueError('inventory.unknown-check')
    observations={}
    executable=None
    binary_digest=None
    for name,argv in commands.items():
        if name=='skills':
            if executable is None:
                observations[name]={'state':'UNKNOWN','reason':'CLI build identity unavailable'}
                continue
            argv[-1]=str(executable)
        print('Running '+name,flush=True)
        try:
            result=subprocess.run(argv,cwd=ROOT,text=True,capture_output=True,timeout=600)
            state='PASS' if result.returncode==0 else 'FAIL'
            observations[name]={'state':state,'argv':argv,'exit_code':result.returncode,'stdout':result.stdout,'stderr':result.stderr}
            if name=='build' and result.returncode==0:
                executable=built_executable(result.stdout)
                binary_digest=hashlib.sha256(executable.read_bytes()).hexdigest()
        except ValueError as error:
            observations[name]={'state':'FAIL','argv':argv,'reason':str(error)}
        except subprocess.TimeoutExpired:
            observations[name]={'state':'UNKNOWN','argv':argv,'reason':'command timeout; no completion observed'}
        except OSError as error:
            observations[name]={'state':'UNKNOWN','argv':argv,'reason':str(error)}
    sources=source_inventory(ROOT,args.output.resolve())
    observations['source_stability']=source_observation(before,sources)
    binary_after=hashlib.sha256(executable.read_bytes()).hexdigest() if executable and executable.is_file() else None
    observations['binary_stability']={'state':'PASS' if binary_digest is not None and binary_digest==binary_after else 'FAIL'}
    source_digest=hashlib.sha256(json.dumps(before,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    passed=all(o['state']=='PASS' for o in observations.values())
    result={'schema':'oh.war/phase1-observation/v1','host':platform.system(),
            'source_digest':source_digest,'source_files':before,'source_files_after':sources,'checks':observations,
            'build_identity':identity,'architecture':platform.machine(),
            'executable':str(executable),'executable_sha256':binary_digest,
            'current_host_passed':passed,'case_assignments':value['cases'],
            'phase_exit_established':False,'qualification_established':False,
            'remaining':['Other required operating system observation with matching source inventory',
                         'Required repository gate and independent phase review',
                         'Real harness/model quality and human effort are not measured by fixed artifacts']}
    with args.output.open('x') as out:json.dump(result,out,indent=2);out.write('\n')
    print(json.dumps({'current_host_passed':passed,'host':result['host'],'phase_exit_established':False,'receipt':str(args.output)}))
    return 0 if passed else 1


if __name__=='__main__':sys.exit(main())
