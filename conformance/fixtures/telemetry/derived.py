# SPDX-License-Identifier: Apache-2.0
# Disposable public CLI controls. No authority, model call or actual user timing.
from pathlib import Path
import json, os, shutil, subprocess, sys
repo, binary, temporary = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
root = temporary / 'repository'
shutil.copytree(repo / 'conformance/fixtures/inbox/repository', root)
env = dict(os.environ, OPENWARRANT_NO_PROJECTS='1', OPENWARRANT_NO_UPDATE_CHECK='1')
def git(*args):
    return subprocess.check_output(['git', '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', '-c', 'commit.gpgsign=false', *args], cwd=root, env=env, text=True).strip()
git('init', '-q');git('add', '.');git('commit', '-qm', 'initial fixtures')
first = git('rev-parse', 'HEAD')
git('commit', '--allow-empty', '-qm', 'IX-WAR-0003 tracked work')
commit = git('rev-parse', 'HEAD')
def run(out, subject=commit):
    result = subprocess.run([binary, 'telemetry', '--commit', subject, '--derived', '--out', out, '--json'], cwd=root, env=env, capture_output=True, text=True)
    report = json.loads(result.stdout)
    assert report['schema']=='oh.war/report/v1'
    return report
report = run('derived.json')
assert report['exit_code']==0,report
baseline = report['result']['baseline']
assert baseline['schema']=='oh.war/telemetry-baseline/v2'
assert baseline['commit']==commit
ratio = baseline['derived']['untracked-work rate']['ratio']
assert (ratio['numerator'],ratio['denominator'],ratio['denominator_scale'])==(1,2,1)
assert 'instrumented' in baseline['derived']['human control minutes per accepted WAR']['not_measurable_yet']
assert all(v=='no baseline' for v in baseline['success_metrics'].values())
original = (root/'derived.json').read_bytes()
(root/'docs/warrants/IX-WAR-0003/manifest.toml').write_text('broken current source')
repeated = run('derived.json')
assert repeated['exit_code']==0 and repeated['result']['baseline']==baseline,repeated
assert (root/'derived.json').read_bytes()==original
(root/'retained.json').write_bytes(b'retained prior artifact')
refused = run('retained.json')
assert refused['exit_code']!=0,refused
assert (root/'retained.json').read_bytes()==b'retained prior artifact'
(root/'linked.json').symlink_to('derived.json')
linked = run('linked.json')
assert linked['exit_code']!=0,linked
assert (root/'derived.json').read_bytes()==original
missing = run('missing.json','0'*40)
assert missing['exit_code']!=0 and not (root/'missing.json').exists(),missing
assert any(d['rule']=='telemetry.observation-unavailable' and d['severity']=='unknown' for d in missing['diagnostics']),missing
# The selected committed adoption baseline, not current config, scopes both sides.
(root/'docs/warrants/IX-WAR-0003/manifest.toml').write_text((repo/'conformance/fixtures/inbox/repository/docs/warrants/IX-WAR-0003/manifest.toml').read_text())
config = root/'openwarrant.toml'
config.write_text(config.read_text()+f'\n[adoption]\nbaseline = "{first}"\n')
git('add','.');git('commit','-qm','IX-WAR-0003 adopt scoped history')
adopted = git('rev-parse','HEAD')
scoped = run('scoped.json',adopted)
assert scoped['exit_code']==0,scoped
ratio = scoped['result']['baseline']['derived']['untracked-work rate']['ratio']
assert (ratio['numerator'],ratio['denominator'])==(0,2),scoped
print('exact history ratios, frozen source selection, scoped adoption denominator, honest missing time, no deltas, byte-preserving replay, collision/link refusal and missing-commit UNKNOWN passed')
