# SPDX-License-Identifier: Apache-2.0
"""Exercise current SDK records and preservation against an actual provider result."""
import copy
import hashlib
import json
from pathlib import Path


def exchange(ow, war, payload, audit, run, compile_command, provider_root, temp):
    observations={}
    def sdk(name,operation,expected=None,**fields):
        request={'schema':'oh.war/sdk-request/v1','operation':operation,**fields}
        process=run([str(war),'sdk','--request','-'],ow,json.dumps(request))
        report=json.loads(process['stdout'])
        observations[name]={'request':request,'process':process}
        if expected:
            if process['exit_code']==0 or report['diagnostics'][0]['code']!=expected:
                raise ValueError(name+': required refusal not observed')
            return None
        if process['exit_code'] or report.get('schema')!='oh.war/report/v1':
            raise ValueError(name+': SDK exchange failed')
        return report['result']

    package=json.loads(payload)
    checked=sdk('package','package-check',files=[{'path':k,'hex':bytes(v).hex()} for k,v in package.items()])
    packet=checked['packet']
    if checked['semantic_coverage_established'] is not False:
        raise ValueError('package integrity promoted to semantic coverage')
    task=next(s for s in packet['sources'] if s['source_digest']==packet['task']['source_digest'])
    check_digest='sha256:'+hashlib.sha256((ow/'crates/openwarrant-core/examples/audit_context_package.rs').read_bytes()).hexdigest()
    contract={'warrant_source_digest':task['source_digest'],'sources':packet['sources'],
              'constraints':[entry['ref'] for entry in packet['binding_context'] if entry['required']],
              'expectations':[{'id':'integration:package-integrity','scope':'SDK integrity audit of this portable package only','check_digest':check_digest}]}
    contract_digest=sdk('contract-digest','digest',domain='contract',payload=contract)['digest']
    subject={'warrant':task['document']['id'],'contract_digest':contract_digest,
             'result_digest':audit['root_digest']}
    # A real local observation, without fabricated trust or human signature.
    record=json.loads((ow/'conformance/sdk/records/observation-untrusted.json').read_text())
    record.update(id='integration:package-audit',subject=subject)
    record['actor']={'id':'integration:sdk-consumer','kind':'agent','role':'checker'}
    record['provenance']={'source':'integration:actual-provider-package','authenticity':'unverified'}
    record['payload']['check_id']='integration:package-integrity'
    record['payload']['check_digest']=check_digest
    result=sdk('record','records',records=[json.dumps(record)],subject=subject)
    if result['qualification_established'] or result['record_count']!=1:
        raise ValueError('record authenticity promoted')
    condition={'id':'integration:required-check','action':'execute','stage':'implementation',
               'purpose':'action-gate','timing':'any','requirement':{'passing-check':{
                   'record_id':record['id'],'check_digest':record['payload']['check_digest']}}}
    readiness=sdk('readiness','readiness',records=[json.dumps(record)],subject=subject,
                  conditions=[condition],action='execute',stage='implementation')
    if readiness['evaluation']['action_ready'] or readiness['evaluation']['action_gates'][0]['state']!='unknown':
        raise ValueError('untrusted evidence cleared action gate')
    changed=copy.deepcopy(record);changed['subject']['result_digest']='sha256:'+'0'*64
    sdk('wrong-result','records',expected='record.subject',records=[json.dumps(changed)],subject=subject)

    assurance_fields={'records':[json.dumps(record)],'subject':subject,'contract':json.dumps(contract),
                      'verification_ref':'integration:missing-verifier','acceptance_ref':'integration:missing-human','conditions':[]}
    assurance=sdk('contract-binding','assurance',**assurance_fields)
    findings=assurance['evaluation']['findings']
    if not any(f['id']=='contract' and f['state']=='established' for f in findings) or assurance['qualification_established']:
        raise ValueError('normalized contract binding not established or qualification promoted')
    wrong_domain=copy.deepcopy(assurance_fields)
    wrong_domain['subject']['contract_digest']=audit['basis_digest']
    wrong_record=copy.deepcopy(record);wrong_record['subject']['contract_digest']=audit['basis_digest']
    wrong_domain['records']=[json.dumps(wrong_record)]
    mismatch=sdk('basis-is-not-contract','assurance',**wrong_domain)
    if not any(f['id']=='contract' and f['state']=='unmet' for f in mismatch['evaluation']['findings']):
        raise ValueError('basis digest accepted as normalized contract digest')

    source=(ow/'conformance/sdk/source/task.md').read_bytes()
    # Import an existing legacy record first; preserve all original paths and bytes.
    old=sdk('legacy-import','legacy-import',payload=(ow/'conformance/sdk/legacy/resolved.json').read_text())
    legacy=json.loads(old['payload'])
    files={'task.md':source,**{'archive/'+k:bytes(v) for k,v in legacy['files'].items()}}
    captured=sdk('preserve','legacy-capture',dialect='rc3',
                 adapter='ow-sdk-preservation/1',entry='task.md',
                 files=[{'path':k,'hex':v.hex()} for k,v in files.items()])
    restored=sdk('restore','legacy-import',payload=captured['payload'])
    restored_files={k:bytes(v) for k,v in json.loads(restored['payload'])['files'].items()}
    if restored_files!=files or restored['qualification_established'] or restored['historical_closure_established']:
        raise ValueError('legacy bytes or authority changed')
    corrupt=json.loads(captured['payload']);corrupt['files']['task.md'].append(120)
    sdk('corrupt-preservation','legacy-import',expected='legacy.integrity',payload=json.dumps(corrupt))
    directory=Path(temp)/'restored';directory.mkdir()
    # Only the known task file crosses into the provider filesystem capability.
    (directory/'task.md').write_bytes(restored_files['task.md'])
    argv=list(compile_command);argv[-3]=str(directory)
    recompiled=run(argv,provider_root)
    observations['recompile_preserved_source']=recompiled
    if recompiled['exit_code'] or recompiled['stdout']!=payload:
        raise ValueError('preserved source changed compiler output')
    return {'passed':True,'observations':observations,'subject':subject,'normalized_contract':contract,'basis_digest':audit['basis_digest'],
            'legacy_inventory_preserved':True,'provider_roundtrip_identical':True,
            'readiness_established':False,'qualification_established':False}
