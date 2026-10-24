#!/usr/bin/env python3
"""Complete only the failed cache gate, preserving all successful source receipts."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import time

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[2]
spec=importlib.util.spec_from_file_location('frozen_qualification',OUT/'validate.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
sha=m.sha


def run(name,command):
    print('START',name,flush=True);start=time.time();log=OUT/'logs'/f'{name}-verified-final.log'
    with log.open('w') as stream:
        p=subprocess.run(command,cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
    assert p.returncode==0,(name,p.returncode)
    counts=[tuple(map(int,x)) for x in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log.read_text())]
    if command[:2]==['cargo','test']:assert counts and all(b==0 for a,b,c in counts)
    print('DONE',name,counts,flush=True)
    return dict(name=name,command=command,log=str(log.relative_to(OUT)),exitCode=p.returncode,elapsedSeconds=round(time.time()-start,3),reused=False,testResults=[dict(passed=a,failed=b,ignored=c) for a,b,c in counts])


def main():
    old,previous=m.validate_dependency_disjoint_patch()
    guard=json.loads((OUT/'native-inputs.json').read_text())
    rows=m.current_inputs(old)
    assert rows==guard['inputs'] and m.digest(rows)==guard['inputsSha256']
    result=json.loads((OUT/'summary.json').read_text())
    assert result['qualifiedCodeRevision']==guard['qualifiedCodeRevision']
    checks={c['name']:c for c in result['checks']}
    assert set(checks)=={name for name,_ in m.FRESH}
    assert all(c['exitCode']==0 for n,c in checks.items() if n!='no-codec-libraries')
    failed=checks['no-codec-libraries']
    assert failed['exitCode']!=0
    result['initialCacheFailure']={**failed,'logSha256':sha(OUT/failed['log']),
        'observedCause':'Cargo reused no-codec cad-predicates metadata whose dependency information omitted the new quadric module despite its unconditional source export',
        'recovery':'cargo clean -p cad-predicates --profile dev; regenerate metadata and recheck; no source edits',
        'cleanLog':'/private/tmp/osv-implicit-sphere-stale-predicate-cache-clean-20261016.log'}
    checks['no-codec-libraries']=run('no-codec-libraries',failed['command'])
    # Refresh this small exact-kernel test suite after repairing its dev cache.
    checks['exact-predicates']=run('exact-predicates',m.BASE+['-p','cad-predicates','--lib'])
    target=Path(os.environ['CARGO_TARGET_DIR'])/'debug/build/cad-predicates'
    variants=[]
    for directory in target.iterdir():
        fp=directory/'fingerprint/lib-cad_predicates.json'
        if not fp.is_file() or json.loads(fp.read_text())['features']!='[]':continue
        deps=list((directory/'out').glob('*.d'))
        assert deps and all('bezier_quadric_identity.rs' in p.read_text() for p in deps)
        variants.append(dict(configuration=directory.name,quadricDependencyPresent=True))
    assert variants
    result['regeneratedNoCodecMetadata']=variants
    result['checks']=list(checks.values())
    for name in m.REUSE:
        if name=='exact-predicates':continue
        c=dict(previous[name]);assert c['exitCode']==0
        log=m.PREV/c['log']
        c.update(reused=True,verifiedDependencyDisjointTestOnlyPatch=True,
                 log=str(Path('..')/m.PREV.name/c['log']),logSha256=sha(log),originalCodeRevision=m.CODE)
        result['checks'].append(c)
    assert len(result['checks'])==14 and all(c['exitCode']==0 for c in result['checks'])
    predecessor=json.loads((m.PREV/'summary.json').read_text())
    result.update(predecessorSourceGuardVerified=predecessor['sourceGuardVerified'],
                  predecessorGuardDifferenceReconciledAsExactReviewedTestOnlyPatch=True,
                  reusedSameProductionDependencyDisjointGates=6)
    result['reuseEvidence']=[dict(path=str(p.relative_to(ROOT)),sha256=sha(p)) for p in [m.PREV/'native-inputs.json',m.PREV/'summary.json']]
    assert rows==m.current_inputs(old)
    for c in result['checks']:
        if c['reused']:assert sha(OUT/c['log'])==c['logSha256']
    result.update(sourceGuardVerified=True,nativeCheckpointPassed=True,originalSphereGlobalG2Qualified=True,
                  previousRationalCapMatrixQualified=True,independentStepQualified=True,
                  originalSphereScope='four specified original sphere STEP cases plus typed signed-chart/mutation/invalid-input/budget tests',
                  previousRationalCapScope='six specified original rational frame and filled annular cap fixtures',
                  totalPassed=sum(t['passed'] for c in result['checks'] for t in c['testResults']),
                  totalIgnored=sum(t['ignored'] for c in result['checks'] for t in c['testResults']))
    result['completionTool']={'path':str(Path(__file__).relative_to(ROOT)),'sha256':sha(Path(__file__))}
    result['fullGoalComplete']=False
    (OUT/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
    print('SOURCE GUARD',True,'TOTAL',result['totalPassed'],'ignored',result['totalIgnored'],flush=True)


if __name__=='__main__':main()
