import hashlib, json, os, pathlib, subprocess, sys, tempfile, time, shutil
keeper=pathlib.Path(__file__).parent
before,after=sys.argv[1:]
work=pathlib.Path('/mnt/4tb/tmp/openwarrant-ow130-retry-fixture')
work.mkdir(exist_ok=False)
os.environ['OPENWARRANT_NO_PROJECTS']='1'
def run(argv,name):
    r=subprocess.run(argv,capture_output=True,text=True,timeout=60)
    (keeper/(name+'.log')).write_text(r.stdout+r.stderr)
    return r
try:
    run(['git','init','-q',str(work)],'retry-init-git')
    r=run([after,'init','--program','Historical retry fixture','--namespace','IA','--root',str(work)],'retry-init')
    assert r.returncode==0,r.stderr
    body=(keeper/'historical-response.toml').read_text()
    response=work/'response.toml';response.write_text(body)
    record=work/'docs/warrants/IA-WAR-0001/verifications/OBL-001.toml'
    args=['--root',str(work),'verify','IA-WAR-0001','--response',str(response)]
    first=run([after,*args],'retry-first')
    assert first.returncode==0 and 'verify.recorded' in first.stdout,first.stdout+first.stderr
    digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
    def tree_hash():
        items=[(str(p.relative_to(record.parent.parent)),digest(p)) for p in sorted(record.parent.parent.rglob('*')) if p.is_file()]
        return hashlib.sha256(json.dumps(items,separators=(',',':')).encode()).hexdigest()
    original_tree=tree_hash()
    original=record.read_bytes(); original_hash=digest(record); original_mtime=record.stat().st_mtime_ns
    journal=record.parent.parent/'journal.jsonl'; journal_bytes=journal.read_bytes()
    time.sleep(.1)
    red=run([before,*args],'retry-before')
    rewritten=record.stat().st_mtime_ns!=original_mtime
    assert red.returncode!=0 and 'already journalled' in red.stdout+red.stderr and rewritten,red.stdout+red.stderr
    record.write_bytes(original);os.utime(record,ns=(original_mtime,original_mtime))
    assert journal.read_bytes()==journal_bytes
    time.sleep(.1)
    green=run([after,*args],'retry-after')
    unchanged=(record.read_bytes()==original and record.stat().st_mtime_ns==original_mtime and journal.read_bytes()==journal_bytes and tree_hash()==original_tree)
    assert green.returncode==0 and 'verify.replayed' in green.stdout and unchanged,green.stdout+green.stderr
    result={'scope':'same literal v1 fixture response and same scratch Warrant, historical before/after binaries','before':{'exit_code':red.returncode,'record_rewritten':rewritten,'named_duplicate_refusal':True},'after':{'exit_code':green.returncode,'record_and_journal_unchanged':unchanged,'whole_warrant_tree_unchanged':tree_hash()==original_tree,'named_replay':True},'fixture_response_sha256':digest(response),'initial_record_sha256':original_hash,'initial_warrant_tree_sha256':original_tree,'binary_sha256':{k:digest(pathlib.Path(v)) for k,v in [('before',before),('after',after)]},'independent_verification':False,'qualification':False}
    (keeper/'historical-retry-observation.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))
finally:
    shutil.rmtree(work)
