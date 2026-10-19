#!/usr/bin/env python3
"""Finish a dependency-disjoint test-only correction; never reuse changed geometry."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
PREV = ROOT/'docs/qualification/implicit-sphere-source-2026-10-16'
CODE = '6476dc11c7091e42577541d46fe2dfe8f356177d'
PATCH_PATH = 'crates/geometry-bridge/src/cad_face_contacts.rs'
OLD = '        assert_eq!(report["spans"], json!(968));'
NEW = '''        let required = field::<usize>(&report, "spans").unwrap();
        assert!(required > 0 && required <= 1000);
        let accounted = report["faces"].as_array().unwrap().iter()
            .map(|f| field::<usize>(&f["result"], "spans").unwrap()).sum::<usize>();
        assert_eq!(required, accounted);
        q["maxSpans"] = json!(required - 1);
        let short = crate::dispatch(q.clone()).unwrap();
        assert_eq!(short["allFacesInjective"], json!(false));
        assert_eq!(field::<usize>(&short, "spans").unwrap(), required - 1);'''
REUSE = ['exact-predicates', 'nurbs-full', 'source-volume-matrices', 'regularity-and-g2', 'brep-full', 'brep-aag-serial', 'cad-step-full']
BASE = ['cargo', 'test', '--locked', '--manifest-path', 'crates/Cargo.toml']
FRESH = [
    ('nurbs-domain-map', ['node', 'scripts/map-nurbs-api.mjs', '--check']),
    ('algorithm-owner-audit', ['node', 'scripts/audit-algorithm-owners.mjs', '--check']),
    ('bridge-full', BASE + ['-p', 'geometry-bridge', '--lib']),
    ('native-json-step', BASE + ['-p', 'geometry-bridge', '--test', 'general_boundary_certificate', '--test', 'closed_material_certificate', '--test', 'smooth_closed_material_certificate', '--test', 'smooth_closed_step_matrix', '--test', 'rational_closed_material_certificate', '--test', 'rational_closed_step_matrix', '--test', 'implicit_sphere_source_certificate', '--test', 'implicit_sphere_step_matrix', '--', '--nocapture']),
    ('independent-rational-cap-step', ['/private/tmp/osv-rational-occt-20261016/bin/python', 'scripts/verify-rational-filled-cap-step.py', str(OUT/'rational-cap-step'), str(OUT/'rational-cap-occt-report.json')]),
    ('independent-implicit-sphere-step', ['/private/tmp/osv-rational-occt-20261016/bin/python', 'scripts/verify-implicit-sphere-step.py', str(OUT/'sphere-step'), str(OUT/'sphere-occt-report.json')]),
    ('no-codec-libraries', ['cargo', 'check', '--locked', '--manifest-path', 'crates/Cargo.toml', '-p', 'brep-core', '-p', 'nurbs-intersect', '-p', 'brep-intersect', '--no-default-features']),
]


def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def digest(rows):
    return hashlib.sha256(json.dumps(rows, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def validate_dependency_disjoint_patch():
    guard = json.loads((PREV/'native-inputs.json').read_text())
    result = json.loads((PREV/'summary.json').read_text())
    assert guard['qualifiedCodeRevision'] == result['qualifiedCodeRevision'] == CODE
    assert digest(guard['inputs']) == guard['inputsSha256']
    # The bridge has already finished with the known stale assertion. Its
    # dependency-disjoint BRep binary may still be running. The predecessor
    # guard honestly records this exact test-only edit when it terminates.
    assert guard['rustc'] == subprocess.check_output(['rustc','--version'],cwd=ROOT,text=True).strip()
    changed = [r['path'] for r in guard['inputs'] if sha(ROOT/r['path']) != r['sha256']]
    assert changed == [PATCH_PATH], changed
    before = subprocess.check_output(['git', 'show', CODE+':'+PATCH_PATH], cwd=ROOT).decode()
    after = (ROOT/PATCH_PATH).read_text()
    assert before.count(OLD) == 1 and before.index(OLD) > before.index('#[cfg(test)]\nmod tests {')
    assert after == before.replace(OLD, NEW), 'Only the exact reviewed test correction is permitted'
    # Reused owners cannot depend on the changed bridge, even for their tests.
    for crate in ['cad-predicates','nurbs-core','brep-core','cad-step']:
        manifest = (ROOT/'crates'/crate/'Cargo.toml').read_text()
        assert 'geometry-bridge' not in manifest and 'geometry_bridge' not in manifest
    checks = {c['name']: c for c in result['checks']}
    assert all(checks[n]['exitCode'] == 0 for n in REUSE if n in checks)
    assert checks['bridge-full']['exitCode'] != 0, 'This successor is specifically a failed test expectation correction'
    current_files = subprocess.check_output(['git','-c','core.fsmonitor=false','ls-files','--cached','--others','--exclude-standard'], cwd=ROOT,text=True).splitlines()
    assert {p for p in current_files if p.startswith('crates/')} == {r['path'] for r in guard['inputs'] if r['path'].startswith('crates/')}
    return guard, checks


def current_inputs(old):
    paths = {r['path'] for r in old['inputs']} | {str(Path(__file__).relative_to(ROOT))}
    return [{'path':p,'sha256':sha(ROOT/p)} for p in sorted(paths)]


def main():
    old, checks = validate_dependency_disjoint_patch()
    (OUT/'logs').mkdir(exist_ok=True)
    rows = current_inputs(old)
    revision = subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    guard = dict(schema='implicit-sphere-test-contract-inputs/1', qualifiedCodeRevision=revision,
                 inputCount=len(rows), inputsSha256=digest(rows), inputs=rows,
                 predecessorCodeRevision=CODE, predecessorInputsSha256=old['inputsSha256'],
                 testOnlyChangedPath=PATCH_PATH, productionInputsUnchanged=True,
                 rustc=subprocess.check_output(['rustc','--version'],cwd=ROOT,text=True).strip())
    (OUT/'native-inputs.json').write_text(json.dumps(guard,indent=2)+'\n')
    r = dict(schema='implicit-sphere-test-contract-qualification/1', qualifiedCodeRevision=revision,
             inputsSha256=guard['inputsSha256'], sourceGuardVerified=False, nativeCheckpointPassed=False,
             reusedLowerFeatureChecks=0, reusedSameProductionDependencyDisjointGates=len(REUSE),
             originalSphereGlobalG2Qualified=False, previousRationalCapMatrixQualified=False,
             independentStepQualified=False, privateReleaseWasmRuntimeQualified=False,
             publicWasmRebuilt=False, browserQualified=False, semanticG1MatrixQualified=False,
             freshCiQualified=False, fullGoalComplete=False, checks=[])
    def save():
        (OUT/'summary.json').write_text(json.dumps(r,indent=2)+'\n')
    save()
    env=dict(os.environ,OSV_RATIONAL_CAP_STEP_DIR=str(OUT/'rational-cap-step'),OSV_IMPLICIT_SPHERE_STEP_DIR=str(OUT/'sphere-step'))
    for name, command in FRESH:
        print('START',name,flush=True);start=time.time();log=OUT/'logs'/f'{name}-final.log'
        with log.open('w') as stream:
            run=subprocess.run(command,cwd=ROOT,env=env,stdout=stream,stderr=subprocess.STDOUT)
        counts=[tuple(map(int,m)) for m in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log.read_text())]
        code=run.returncode or (3 if command[:2]==['cargo','test'] and not counts else 0)
        r['checks'].append(dict(name=name,command=command,log=str(log.relative_to(OUT)),exitCode=code,elapsedSeconds=round(time.time()-start,3),reused=False,testResults=[dict(passed=a,failed=b,ignored=c) for a,b,c in counts]))
        save();print('DONE',name,'exit',code,'tests',counts,flush=True)
        if code:return code
    print('WAIT predecessor BRep terminal receipt',flush=True)
    while True:
        predecessor=json.loads((PREV/'summary.json').read_text())
        if any(c['name']=='brep-full' for c in predecessor['checks']):break
        time.sleep(10)
    old,checks=validate_dependency_disjoint_patch()
    assert all(checks[n]['exitCode']==0 for n in REUSE)
    r['predecessorSourceGuardVerified']=predecessor['sourceGuardVerified']
    r['predecessorGuardDifferenceReconciledAsExactReviewedTestOnlyPatch']=True
    for name in REUSE:
        c=dict(checks[name]);log=PREV/c['log']
        c.update(reused=True,verifiedDependencyDisjointTestOnlyPatch=True,
                 log=str(Path('..')/PREV.name/c['log']),logSha256=sha(log),originalCodeRevision=CODE)
        r['checks'].append(c)
    r['reuseEvidence']=[{'path':str(p.relative_to(ROOT)),'sha256':sha(p)} for p in [PREV/'native-inputs.json',PREV/'summary.json']]
    r['sourceGuardVerified']=rows==current_inputs(old)
    # Reused logs and original compiled-source receipts must remain immutable.
    for c in r['checks']:
        if c['reused']:assert sha(OUT/c['log'])==c['logSha256']
    for receipt in r['reuseEvidence']:assert sha(ROOT/receipt['path'])==receipt['sha256']
    passed=r['sourceGuardVerified']
    r.update(nativeCheckpointPassed=passed,originalSphereGlobalG2Qualified=passed,
             previousRationalCapMatrixQualified=passed,independentStepQualified=passed,
             originalSphereScope='four STEP fixtures plus typed signed-chart/mutation/invalid-input/budget cases',
             previousRationalCapScope='six specified original rational frame and filled annular cap fixtures',
             totalPassed=sum(t['passed'] for c in r['checks'] for t in c['testResults']),
             totalIgnored=sum(t['ignored'] for c in r['checks'] for t in c['testResults']))
    save();print('SOURCE GUARD',passed,'TOTAL',r['totalPassed'],'ignored',r['totalIgnored'],flush=True)
    return 0 if passed else 2


if __name__=='__main__':raise SystemExit(main())
