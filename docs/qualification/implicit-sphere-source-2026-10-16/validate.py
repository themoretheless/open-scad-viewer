#!/usr/bin/env python3
"""Frozen native source/cap/sphere checks; separate release and UI gates."""
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
LOWER = '530c7bd3b'
BASE = ['cargo', 'test', '--locked', '--manifest-path', 'crates/Cargo.toml']
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
    ('native-json-step', BASE + ['-p', 'geometry-bridge', '--test', 'general_boundary_certificate', '--test', 'closed_material_certificate', '--test', 'smooth_closed_material_certificate', '--test', 'smooth_closed_step_matrix', '--test', 'rational_closed_material_certificate', '--test', 'rational_closed_step_matrix', '--test', 'implicit_sphere_source_certificate', '--test', 'implicit_sphere_step_matrix', '--', '--nocapture']),
    ('independent-rational-cap-step', ['/private/tmp/osv-rational-occt-20261016/bin/python', 'scripts/verify-rational-filled-cap-step.py', str(OUT/'rational-cap-step'), str(OUT/'rational-cap-occt-report.json')]),
    ('independent-implicit-sphere-step', ['/private/tmp/osv-rational-occt-20261016/bin/python', 'scripts/verify-implicit-sphere-step.py', str(OUT/'sphere-step'), str(OUT/'sphere-occt-report.json')]),
    ('no-codec-libraries', ['cargo', 'check', '--locked', '--manifest-path', 'crates/Cargo.toml', '-p', 'brep-core', '-p', 'nurbs-intersect', '-p', 'brep-intersect', '--no-default-features']),
]
TOOLS = {'rust-toolchain.toml', 'scripts/verify-rational-filled-cap-step.py', 'scripts/verify-implicit-sphere-step.py', 'scripts/map-nurbs-api.mjs', 'scripts/audit-algorithm-owners.mjs', 'docs/design/nurbs-api-domains.json', 'docs/design/original-implicit-sphere-certificate.md', str(Path(__file__).relative_to(ROOT)), str(OUT.relative_to(ROOT)/'wasm-runtime.mts')}


def inputs():
    files = subprocess.check_output(['git', '-c', 'core.fsmonitor=false', 'ls-files', '--cached', '--others', '--exclude-standard'], cwd=ROOT, text=True).splitlines()
    return [{'path': p, 'sha256': hashlib.sha256((ROOT/p).read_bytes()).hexdigest()} for p in sorted(set(files)) if p.startswith('crates/') or p in TOOLS]


def digest(rows):
    return hashlib.sha256(json.dumps(rows, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def run_check(name, command, env):
    print('START', name, flush=True)
    start = time.time()
    log = OUT/'logs'/f'{name}-final.log'
    with log.open('w') as stream:
        run = subprocess.run(command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT)
    counts = [tuple(map(int, m)) for m in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', log.read_text())]
    code = run.returncode
    if command[:2] == ['cargo', 'test'] and not counts:
        code = code or 3
    report = dict(name=name, command=command, log=str(log.relative_to(OUT)), exitCode=code,
                  elapsedSeconds=round(time.time()-start, 3),
                  testResults=[dict(passed=a, failed=b, ignored=c) for a,b,c in counts])
    print('DONE', name, 'exit', code, 'tests', counts, flush=True)
    return report


def main():
    if sys.argv[1:]:
        raise ValueError('No lower-check reuse for changed geometry or source ownership')
    (OUT/'logs').mkdir(exist_ok=True)
    rows = inputs()
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    guard = dict(schema='original-implicit-sphere-native-inputs/1', qualifiedCodeRevision=revision,
                 lowerCheckpointRevision=LOWER, inputCount=len(rows), inputsSha256=digest(rows), inputs=rows,
                 digestEncoding='sha256 of UTF-8 compact sort_keys JSON of sorted inputs list',
                 rustc=subprocess.check_output(['rustc', '--version'], cwd=ROOT, text=True).strip())
    (OUT/'native-inputs.json').write_text(json.dumps(guard, indent=2)+'\n')
    result = dict(schema='original-implicit-sphere-native-qualification/1', qualifiedCodeRevision=revision,
                  lowerCheckpointRevision=LOWER, sourceGuardVerified=False, nativeCheckpointPassed=False,
                  inputsSha256=guard['inputsSha256'], reusedLowerChecks=0, checks=[],
                  originalSphereGlobalG2Qualified=False, previousRationalCapMatrixQualified=False,
                  independentStepQualified=False, privateReleaseWasmRuntimeQualified=False,
                  publicWasmRebuilt=False, browserQualified=False, semanticG1MatrixQualified=False,
                  freshCiQualified=False, fullGoalComplete=False)
    def save():
        (OUT/'summary.json').write_text(json.dumps(result, indent=2)+'\n')
    save()
    env = dict(os.environ, OSV_RATIONAL_CAP_STEP_DIR=str(OUT/'rational-cap-step'),
               OSV_IMPLICIT_SPHERE_STEP_DIR=str(OUT/'sphere-step'))
    # Independent native test binaries: the long BRep suite overlaps the
    # remaining sequential gates. AAG itself retains one test thread.
    with ThreadPoolExecutor(max_workers=2) as pool:
        long = pool.submit(run_check, *CHECKS[0], env)
        for name, command in CHECKS[1:]:
            report = run_check(name, command, env)
            result['checks'].append(report)
            save()
            if report['exitCode']:
                # Preserve and wait for the already-authorized long check.
                break
        result['checks'].append(long.result())
    complete = len(result['checks']) == len(CHECKS) and all(c['exitCode'] == 0 for c in result['checks'])
    result['sourceGuardVerified'] = rows == inputs()
    passed = complete and result['sourceGuardVerified']
    result.update(nativeCheckpointPassed=passed, originalSphereGlobalG2Qualified=passed,
                  previousRationalCapMatrixQualified=passed, independentStepQualified=passed,
                  originalSphereScope='four STEP fixtures plus typed signed-chart, mutation, malformed-input and short-budget tests',
                  previousRationalCapScope='six specified original rational frame and filled annular cap fixtures',
                  totalPassed=sum(t['passed'] for c in result['checks'] for t in c['testResults']),
                  totalIgnored=sum(t['ignored'] for c in result['checks'] for t in c['testResults']))
    save()
    print('SOURCE GUARD', result['sourceGuardVerified'], 'TOTAL', result['totalPassed'], 'ignored', result['totalIgnored'], 'PASSED', passed, flush=True)
    return 0 if passed else 2


if __name__ == '__main__':
    raise SystemExit(main())
