#!/usr/bin/env python3
"""Structural skill/discovery audit only; this does not simulate a model."""
from pathlib import Path
import json,re,sys
ROOT=Path(__file__).resolve().parents[2]
SKILLS=ROOT/'.claude/skills'
NAMES={'war','openwarrant','war-grill','war-spec','war-tickets','war-map','war-review','war-migrate'}

def check():
    links=0
    words=0
    for name in sorted(NAMES):
        p=SKILLS/name/'SKILL.md';s=p.read_text();head=s.split('---',2)[1]
        assert f'name: {name}\n' in head,name
        assert 'description:' in head,name
        assert 'disable-model-invocation: true' not in head,name
        assert '\u2014' not in s,name
        assert len(s.splitlines())<=120,name
        words+=len(s.split())
        link=ROOT/'.agents/skills'/name
        assert link.is_symlink() and link.resolve()==p.parent.resolve(),name
        for target in re.findall(r'\]\(([^)]+)\)',s):
            if '://' not in target and not target.startswith('#'):
                assert (p.parent/target.split('#')[0]).is_file(),(name,target)
                links+=1
    for p in (SKILLS/'openwarrant/references').glob('*.md'):
        for target in re.findall(r'\]\(([^)]+)\)',p.read_text()):
            if '://' not in target and not target.startswith('#'):
                assert (p.parent/target.split('#')[0]).is_file(),(p,target)
    license=(SKILLS/'openwarrant/LICENSE.mattpocock').read_text()
    assert 'Copyright (c) 2026 Matt Pocock' in license and 'MIT License' in license
    prompts=json.loads((Path(__file__).parent/'prompts.json').read_text())
    assert len(prompts)==17 and len({x['prompt'] for x in prompts})==17
    assert {x['route'] for x in prompts}=={'progress','war-grill','war-spec','war-tickets','war-map','war-review','war-migrate','execution','none'}
    print(json.dumps({'status':'PASS','scope':'structural-discovery-links-not-model-behavior','skills':len(NAMES),'links':links,'skill_words':words,'prompt_cases_prepared':len(prompts),'model_prompt_evaluation':'NOT RUN','harness_activation':'NOT RUN'},sort_keys=True))

if __name__=='__main__':
    try:check()
    except (AssertionError,OSError,ValueError) as e:
        print('FAIL:',e,file=sys.stderr);sys.exit(1)
