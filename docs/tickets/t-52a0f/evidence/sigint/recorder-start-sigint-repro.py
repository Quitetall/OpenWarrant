# SPDX-License-Identifier: Apache-2.0
import json, os, subprocess, sys, tempfile
from pathlib import Path
source=Path(sys.argv[1])
with tempfile.TemporaryDirectory(dir="/mnt/4tb/tmp/openwarrant-conformance-continuation-runtime/tmp") as temporary:
 root=Path(temporary); meta=root/"meta.json"; out=root/"out.json"
 meta.write_text(json.dumps({"participant":"P1","build_commit":"a"*40,"inputs_sha256":"b"*64,"scenario":"synthetic startup signal control; no participant evidence","consent_confirmed":True,"assisted":False,"failures":[]}))
 child = """import os, signal, sys
sys.path.insert(0, sys.argv[1])
import record
original = record.os.fsync
calls = 0
def sync(fd):
 global calls
 calls += 1
 original(fd)
 if calls == 1:
  os.kill(os.getpid(), signal.SIGINT)
record.os.fsync = sync
sys.argv = ['record.py', '--metadata', sys.argv[2], '--out', sys.argv[3]]
raise SystemExit(record.main())
"""
 result=subprocess.run([sys.executable,"-c",child,str(source),str(meta),str(out)],input=b"",capture_output=True,timeout=10)
 summary={"exit":result.returncode,"output_retained":out.exists(),"stderr":result.stderr.decode()[-2500:]}
 if out.exists(): summary["record"]=json.loads(out.read_text())
 print(json.dumps(summary,indent=2))
 assert result.returncode == 1 and out.exists(), "startup SIGINT escaped without retained unknown session"
 assert summary["record"]["outcome"] == "unknown" and summary["record"]["termination"] == "signal-interruption"
