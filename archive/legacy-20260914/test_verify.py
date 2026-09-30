#!/usr/bin/env python3
"""Black-box archive integrity regressions, including optimized Python."""
import hashlib,io,json,shutil,subprocess,tarfile,tempfile
from pathlib import Path
archive=Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
with tempfile.TemporaryDirectory(prefix='ow-archive-regression-') as t:
 root=Path(t);(root/'original').mkdir();(root/'original/doc.md').write_bytes(b'original\n')
 shutil.copyfile(archive/'history.bundle',root/'history.bundle')
 with tarfile.open(root/'snapshot.tar.gz','w:gz') as tar:
  i=tarfile.TarInfo('docs/doc.md');i.size=9;i.mode=0o644;tar.addfile(i,io.BytesIO(b'original\n'))
 m={'bundle_sha256':sha((root/'history.bundle').read_bytes()),'baseline_files':[{'kind':'file','path':'doc.md','sha256':sha(b'original\n')}], 'worktrees':[{'archive':'snapshot.tar.gz','archive_sha256':sha((root/'snapshot.tar.gz').read_bytes()),'files':[{'kind':'file','path':'docs/doc.md','mode':0o644,'sha256':sha(b'original\n')}]}]}
 (root/'relocations.json').write_text('[]')
 def save():
  b=json.dumps(m).encode();(root/'manifest.json').write_bytes(b);(root/'manifest.sha256').write_text(sha(b))
 def check(label,expected,needle=''):
  for optimize in ([],['-O']):
   p=subprocess.run(['python3',*optimize,str(archive/'verify.py'),'--root',str(root)],capture_output=True,text=True)
   if p.returncode!=expected or (needle and needle not in p.stderr):raise RuntimeError((label,optimize,p.returncode,p.stdout,p.stderr))
  print('PASS',label,'normal + optimized Python')
 save();check('valid archive',0)
 (root/'original/doc.md').write_bytes(b'corrupt\n');check('changed original refused',1,'doc.md');(root/'original/doc.md').write_bytes(b'original\n')
 (root/'manifest.json').write_text('{}');check('changed manifest refused',1,'Manifest digest mismatch');save()
 with tarfile.open(root/'snapshot.tar.gz','w:gz'):pass
 m['worktrees'][0]['archive_sha256']=sha((root/'snapshot.tar.gz').read_bytes());save();check('missing snapshot member refused',1,'Duplicate or lost entry')
