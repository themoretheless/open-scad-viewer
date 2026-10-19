"""Fresh remaining kernel gates for the immutable guarded Rust source.
Earlier same-source NURBS/JSON/OCCT/WASM receipts are separate; no lower-source
qualification is reused. This runner does not publish or certify the full goal.
"""
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

ROOT = Path(__file__).resolve().parents[3]
OUT = Path('/private/tmp/osv-guided-geometric-remaining-kernel-20261016')
BASE = ['cargo', '+nightly-2026-09-16', 'test', '--locked', '--manifest-path', 'crates/Cargo.toml']
CHECKS = [
    ('nurbs-full', BASE+['-p','nurbs-core','--lib']),
    ('guided-native-json-step', BASE+['-p','geometry-bridge','--test','guided_frame_source_certificate']),
    ('brep-full', BASE+['-p','brep-core','--lib','--','--skip','aag']),
    ('exact-predicates', BASE+['-p','cad-predicates','--lib']),
    ('source-volume-matrices', BASE+['-p','nurbs-core','--test','ideal_volume','--test','ideal_spatial_volume','--test','ideal_volume_atlas','--test','ideal_closed_spatial_volume']),
    ('regularity-and-g2', BASE+['-p','nurbs-core','--test','surface_regularity','--test','surface_g2']),
    ('brep-aag-serial', BASE+['-p','brep-core','--lib','aag','--','--test-threads=1']),
    ('cad-step-full', BASE+['-p','cad-step']),
    ('bridge-full', BASE+['-p','geometry-bridge','--lib']),
    ('legacy-native-json-step', BASE+['-p','geometry-bridge','--test','general_boundary_certificate','--test','closed_material_certificate','--test','smooth_closed_material_certificate','--test','smooth_closed_step_matrix','--test','rational_closed_material_certificate','--test','rational_closed_step_matrix','--test','implicit_sphere_source_certificate','--test','implicit_sphere_step_matrix','--test','fibered_source_certificate','--test','fibered_step_matrix','--test','arc_bishop_source_certificate','--test','general_frame_source_certificate']),
    ('no-codec-libraries', ['cargo','+nightly-2026-09-16','check','--locked','--manifest-path','crates/Cargo.toml','-p','brep-core','-p','nurbs-intersect','-p','brep-intersect','--no-default-features']),
]


def unchanged(guard):
    return all(hashlib.sha256((ROOT/r['path']).read_bytes()).hexdigest()==r['sha256'] for r in guard['inputs'])


def run(item, env):
    name, command = item
    log = OUT/f'{name}.log'
    print('START', name, flush=True)
    start = time.time()
    with log.open('w') as stream:
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT)
    text = log.read_text()
    counts = [dict(passed=int(a),failed=int(b),ignored=int(c)) for a,b,c in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)]
    code = result.returncode
    if 'test' in command[:3] and not counts:
        code = code or 3
    report = dict(name=name,command=command,exitCode=code,elapsedSeconds=round(time.time()-start,3),testResults=counts,log=str(log),logSha256=hashlib.sha256(log.read_bytes()).hexdigest())
    print('DONE',name,code,counts,flush=True)
    return report


def main():
    OUT.mkdir(exist_ok=True)
    guard = json.loads((Path(__file__).parent/'inputs.json').read_text())
    assert unchanged(guard), 'Frozen source changed; do not qualify another revision'
    result = dict(schema='guided-geometric-complete-kernel-gates/1',frozenCodeRevision=guard['revision'],inputsSha256=guard['inputsSha256'],runnerSha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),checks=[],complete=False,passed=False,fullGoalComplete=False)
    def save():
        (OUT/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
    save()
    env = dict(os.environ,CARGO_BUILD_JOBS='2',RUST_TEST_THREADS='2',CARGO_TARGET_DIR='/private/tmp/osv-guided-geometric-seams-target-20261016',OSV_GUIDED_FRAME_STEP_DIR=str(OUT/'guided-frame-step'),OSV_GENERAL_FRAME_STEP_DIR=str(OUT/'general-frame-step'),OSV_ARC_BISHOP_STEP_DIR=str(OUT/'arc-bishop-step'),OSV_FIBERED_STEP_DIR=str(OUT/'fibered-step'),OSV_RATIONAL_CAP_STEP_DIR=str(OUT/'rational-cap-step'),OSV_IMPLICIT_SPHERE_STEP_DIR=str(OUT/'sphere-step'))
    if True:
        for item in CHECKS:
            result['checks'].append(run(item,env));save()
            if result['checks'][-1]['exitCode']:
                break
    result['sourceGuardVerified'] = unchanged(guard)
    result['complete'] = len(result['checks'])==len(CHECKS)
    result['passed'] = result['complete'] and result['sourceGuardVerified'] and all(c['exitCode']==0 for c in result['checks'])
    result['totals'] = {k:sum(r[k] for c in result['checks'] for r in c['testResults']) for k in ['passed','failed','ignored']}
    save()
    print(json.dumps({k:v for k,v in result.items() if k!='checks'}),flush=True)
    return 0 if result['passed'] else 1


if __name__=='__main__':
    raise SystemExit(main())
