#!/usr/bin/env python3
"""Fresh frozen native gates. Never infer a full goal or UI/release result."""
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time
ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
BASE = ['cargo', '+nightly-2026-09-16', 'test', '--locked', '--manifest-path', 'crates/Cargo.toml']
CHECKS = [
    ('brep-full', BASE + ['-p', 'brep-core', '--lib', '--', '--skip', 'aag']),
    ('nurbs-domain-map', ['node', 'scripts/map-nurbs-api.mjs', '--check']),
    ('algorithm-owner-audit', ['node', 'scripts/audit-algorithm-owners.mjs', '--check']),
    ('exact-predicates', BASE + ['-p', 'cad-predicates', '--lib']),
    ('nurbs-full', BASE + ['-p', 'nurbs-core', '--lib']),
    ('source-volume-matrices', BASE + ['-p', 'nurbs-core', '--test', 'ideal_volume', '--test', 'ideal_spatial_volume', '--test', 'ideal_volume_atlas', '--test', 'ideal_closed_spatial_volume', '--', '--nocapture']),
    ('regularity-and-g2', BASE + ['-p', 'nurbs-core', '--test', 'surface_regularity', '--test', 'surface_g2', '--', '--nocapture']),
    ('brep-aag-serial', BASE + ['-p', 'brep-core', '--lib', 'aag', '--', '--test-threads=1']),
    ('cad-step-full', BASE + ['-p', 'cad-step']),
    ('bridge-full', BASE + ['-p', 'geometry-bridge', '--lib']),
    ('native-json-step', BASE + ['-p', 'geometry-bridge', '--test', 'general_boundary_certificate', '--test', 'closed_material_certificate', '--test', 'smooth_closed_material_certificate', '--test', 'smooth_closed_step_matrix', '--test', 'rational_closed_material_certificate', '--test', 'rational_closed_step_matrix', '--test', 'implicit_sphere_source_certificate', '--test', 'implicit_sphere_step_matrix', '--test', 'fibered_source_certificate', '--test', 'fibered_step_matrix', '--', '--nocapture']),
    ('independent-fibered-step', ['/private/tmp/osv-rational-occt-20261016/bin/python', 'scripts/verify-fibered-source-step.py', str(OUT/'fibered-step'), str(OUT/'fibered-occt-report.json')]),
    ('independent-rational-cap-step', ['/private/tmp/osv-rational-occt-20261016/bin/python', 'scripts/verify-rational-filled-cap-step.py', str(OUT/'rational-cap-step'), str(OUT/'rational-cap-occt-report.json')]),
    ('independent-implicit-sphere-step', ['/private/tmp/osv-rational-occt-20261016/bin/python', 'scripts/verify-implicit-sphere-step.py', str(OUT/'sphere-step'), str(OUT/'sphere-occt-report.json')]),
    ('no-codec-libraries', ['cargo', '+nightly-2026-09-16', 'check', '--locked', '--manifest-path', 'crates/Cargo.toml', '-p', 'brep-core', '-p', 'nurbs-intersect', '-p', 'brep-intersect', '--no-default-features']),
]
TOOLS = {'rust-toolchain.toml', 'scripts/verify-rational-filled-cap-step.py', 'scripts/verify-implicit-sphere-step.py', 'scripts/verify-fibered-source-step.py', 'scripts/map-nurbs-api.mjs', 'scripts/audit-algorithm-owners.mjs', 'docs/design/nurbs-api-domains.json', 'docs/design/original-fibered-source-body-certificate.md', str(Path(__file__).relative_to(ROOT)), str(OUT.relative_to(ROOT)/'wasm-runtime.mts'), 'src/services/valueBinaryCodec.ts'}
def inputs():
    files = subprocess.check_output(['git','-c','core.fsmonitor=false','ls-files','--cached','--others','--exclude-standard'],cwd=ROOT,text=True).splitlines()
    return [{'path':p,'sha256':hashlib.sha256((ROOT/p).read_bytes()).hexdigest()} for p in sorted(set(files)) if p.startswith('crates/') or p in TOOLS]
def digest(rows):
    return hashlib.sha256(json.dumps(rows,sort_keys=True,separators=(',',':')).encode()).hexdigest()
def run_check(name,command,env):
    print('START',name,flush=True);start=time.time();log=OUT/'logs'/f'{name}.log'
    with log.open('w') as stream:
        process=subprocess.run(command,cwd=ROOT,env=env,stdout=stream,stderr=subprocess.STDOUT)
    counts=[tuple(map(int,m)) for m in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log.read_text())]
    code=process.returncode
    if 'test' in command[:3] and not counts: code=code or 3
    result=dict(name=name,command=command,log=str(log.relative_to(OUT)),logSha256=hashlib.sha256(log.read_bytes()).hexdigest(),exitCode=code,elapsedSeconds=round(time.time()-start,3),testResults=[dict(passed=a,failed=b,ignored=c) for a,b,c in counts])
    print('DONE',name,'exit',code,'tests',counts,flush=True);return result
def main():
    (OUT/'logs').mkdir(exist_ok=True)
    rows=inputs();revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    dirty=subprocess.check_output(['git','-c','core.fsmonitor=false','status','--porcelain','--','crates','scripts','docs/design',str(OUT.relative_to(ROOT)/'validate.py'),str(OUT.relative_to(ROOT)/'wasm-runtime.mts')],cwd=ROOT,text=True).strip()
    if dirty:raise ValueError('Freeze source, qualification code and runtime harness before running native gates: '+dirty)
    guard=dict(schema='original-fibered-source-native-inputs/1',qualifiedCodeRevision=revision,inputCount=len(rows),inputsSha256=digest(rows),inputs=rows,rustc=subprocess.check_output(['rustc','+nightly-2026-09-16','--version'],cwd=ROOT,text=True).strip())
    (OUT/'native-inputs.json').write_text(json.dumps(guard,indent=2)+'\n')
    result=dict(schema='original-fibered-source-native-qualification/1',qualifiedCodeRevision=revision,sourceGuardVerified=False,nativeCheckpointPassed=False,inputsSha256=guard['inputsSha256'],reusedLowerChecks=0,checks=[],originalFiberedPlanarCapG2Qualified=False,previousSphereAndRationalCapMatricesQualified=False,independentStepQualified=False,privateReleaseWasmRuntimeQualified=False,publicWasmRebuilt=False,browserQualified=False,semanticG1MatrixQualified=False,freshCiQualified=False,fullGoalComplete=False)
    def save():(OUT/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
    save()
    env=dict(os.environ,CARGO_TARGET_DIR='/private/tmp/osv-source-fibered-caps-target-20261016',OSV_RATIONAL_CAP_STEP_DIR=str(OUT/'rational-cap-step'),OSV_IMPLICIT_SPHERE_STEP_DIR=str(OUT/'sphere-step'),OSV_FIBERED_STEP_DIR=str(OUT/'fibered-step'),PYTHONPYCACHEPREFIX='/private/tmp/osv-fibered-pycache-20261016')
    with ThreadPoolExecutor(max_workers=2) as pool:
        long=pool.submit(run_check,*CHECKS[0],env)
        for name,command in CHECKS[1:]:
            result['checks'].append(run_check(name,command,env));save()
            if result['checks'][-1]['exitCode']:break
        result['checks'].append(long.result())
    result['sourceGuardVerified']=rows==inputs()
    passed=len(result['checks'])==len(CHECKS) and all(c['exitCode']==0 for c in result['checks']) and result['sourceGuardVerified']
    result.update(nativeCheckpointPassed=passed,originalFiberedPlanarCapG2Qualified=passed,previousSphereAndRationalCapMatricesQualified=passed,independentStepQualified=passed)
    result['totals']={key:sum(r[key] for c in result['checks'] for r in c['testResults']) for key in ['passed','failed','ignored']}
    save();print(json.dumps({k:v for k,v in result.items() if k!='checks'}),flush=True)
    return 0 if passed else 1
if __name__=='__main__':raise SystemExit(main())
