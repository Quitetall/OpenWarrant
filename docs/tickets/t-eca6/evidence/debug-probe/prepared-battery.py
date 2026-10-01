from pathlib import Path
import subprocess,os,time,json,hashlib
for i in range(2):
 trace=Path('/mnt/2tb')/f'ow-prepared-index-{i}-trace.log';trace.write_text('')
 env=dict(os.environ,GIT_TRACE_PERFORMANCE=str(trace),OPENWARRANT_NO_PROJECTS='1',OPENWARRANT_NO_UPDATE_CHECK='1')
 start=time.monotonic();r=subprocess.run(['/mnt/4tb/tmp/ow-signature-control-runtime/target/debug/war','next','--json'],env=env,capture_output=True)
 rows=trace.read_text().splitlines();json.loads(r.stdout)
 print(json.dumps({'sample':i,'exit':r.returncode,'wall_seconds':round(time.monotonic()-start,3),'diff_calls':sum('git command:' in l and ' diff --name-only' in l for l in rows),'diff_seconds':sum(float(l.split('performance: ')[1].split(' s:')[0]) for l in rows if 'git command:' in l and ' diff --name-only' in l),'output_sha256':hashlib.sha256(r.stdout).hexdigest()}),flush=True)
