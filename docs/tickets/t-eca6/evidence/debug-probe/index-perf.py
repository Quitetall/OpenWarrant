from pathlib import Path
import subprocess,tempfile,time,json,os,hashlib,resource
source=Path('/mnt/4tb/tmp/ow-git-index-performance')
war='/mnt/4tb/tmp/ow-signature-control-runtime/target/debug/war'
results=[]
with tempfile.TemporaryDirectory(prefix='ow-index-perf-') as td:
    root=Path(td)/'r'
    subprocess.run(['git','clone','-q','--local','--no-hardlinks',str(source),str(root)],check=True)
    index=root/'.git/index'
    tree=subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD^{tree}'],text=True).strip()
    files=subprocess.check_output(['git','-C',str(root),'ls-files','-z']).split(b'\0')
    entry_same_second=sum(int((root/os.fsdecode(p)).stat().st_mtime)==int(index.stat().st_mtime) for p in files if p and (root/os.fsdecode(p)).is_file())
    print('Same-second tracked entries:',entry_same_second,flush=True)
    for label in ['fresh','fresh-repeat','after-refresh']:
        if label=='after-refresh':
            subprocess.run(['git','-C',str(root),'update-index','--refresh'],check=True,capture_output=True)
        before=hashlib.sha256(index.read_bytes()).hexdigest()
        trace=Path('/mnt/2tb')/f'ow-index-{label}-git-trace.log'
        trace.write_text('')
        env=dict(os.environ,OPENWARRANT_NO_PROJECTS='1',OPENWARRANT_NO_UPDATE_CHECK='1',GIT_TRACE_PERFORMANCE=str(trace))
        usage=resource.getrusage(resource.RUSAGE_CHILDREN);start=time.monotonic()
        r=subprocess.run([war,'--root',str(root),'next','--json'],env=env,capture_output=True)
        elapsed=time.monotonic()-start;after=resource.getrusage(resource.RUSAGE_CHILDREN)
        json.loads(r.stdout)
        trace_lines=trace.read_text().splitlines()
        row={'sample':label,'exit':r.returncode,'wall_seconds':round(elapsed,3),'child_cpu_seconds':round(after.ru_utime+after.ru_stime-usage.ru_utime-usage.ru_stime,3),'diff_seconds':round(sum(float(l.split('performance: ')[1].split(' s:')[0]) for l in trace_lines if 'git command:' in l and ' diff --name-only' in l),6),'index_unchanged':before==hashlib.sha256(index.read_bytes()).hexdigest(),'output_sha256':hashlib.sha256(r.stdout).hexdigest(),'diff_calls':sum('git command:' in l and ' diff --name-only' in l for l in trace_lines),'untracked_calls':sum('git command:' in l and ' ls-files --others' in l for l in trace_lines)}
        results.append(row);print(json.dumps(row),flush=True)
Path('/mnt/2tb/ow-index-perf-results.json').write_text(json.dumps({'same_second_entries':entry_same_second,'binary_sha256':hashlib.sha256(Path(war).read_bytes()).hexdigest(),'samples':results},indent=2)+'\n')
