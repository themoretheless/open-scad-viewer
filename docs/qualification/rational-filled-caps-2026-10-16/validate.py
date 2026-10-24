#!/usr/bin/env python3
"""Fresh frozen original rational-frame/filled-annular-cap native qualification; no delivery credit."""
import hashlib
import json
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
LOWER = 'b6cab719a'
BASE = ['cargo', 'test', '--locked', '--manifest-path', 'crates/Cargo.toml']
CHECKS = [
    ('nurbs-domain-map', ['node', 'scripts/map-nurbs-api.mjs', '--check']),
    ('algorithm-owner-audit', ['node', 'scripts/audit-algorithm-owners.mjs', '--check']),
    ('exact-predicates', BASE + ['-p', 'cad-predicates', '--lib']),
    ('nurbs-full', BASE + ['-p', 'nurbs-core', '--lib']),
    ('source-volume-matrices', BASE + ['-p', 'nurbs-core', '--test', 'ideal_volume', '--test', 'ideal_spatial_volume', '--test', 'ideal_volume_atlas', '--test', 'ideal_closed_spatial_volume', '--', '--nocapture']),
    ('regularity-and-g2', BASE + ['-p', 'nurbs-core', '--test', 'surface_regularity', '--test', 'surface_g2', '--', '--nocapture']),
    ('brep-full', BASE + ['-p', 'brep-core', '--lib', '--', '--skip', 'aag']),
    ('brep-aag-serial', BASE + ['-p', 'brep-core', '--lib', 'aag', '--', '--test-threads=1']),
    ('bridge-full', BASE + ['-p', 'geometry-bridge', '--lib']),
    ('native-json-step', BASE + ['-p', 'geometry-bridge', '--test', 'general_boundary_certificate', '--test', 'closed_material_certificate', '--test', 'smooth_closed_material_certificate', '--test', 'smooth_closed_step_matrix', '--', '--nocapture']),
    ('no-codec-libraries', ['cargo', 'check', '--locked', '--manifest-path', 'crates/Cargo.toml', '-p', 'brep-core', '-p', 'nurbs-intersect', '-p', 'brep-intersect', '--no-default-features']),
]


def inputs():
    files = subprocess.check_output(['git', '-c', 'core.fsmonitor=false', 'ls-files', '--cached', '--others', '--exclude-standard'], cwd=ROOT, text=True).splitlines()
    return [{'path': p, 'sha256': hashlib.sha256((ROOT / p).read_bytes()).hexdigest()}
            for p in sorted(set(files)) if p.startswith('crates/') or p == 'rust-toolchain.toml']


def digest(rows):
    return hashlib.sha256(json.dumps(rows, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def main():
    (OUT / 'logs').mkdir(exist_ok=True)
    rows = inputs()
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    guard = {'schema': 'rational-filled-cap-native-inputs-v1', 'qualifiedCodeRevision': revision,
             'lowerCheckpointRevision': LOWER, 'digestEncoding': 'sha256 of UTF-8 compact sort_keys JSON of sorted inputs list',
             'inputCount': len(rows), 'inputsSha256': digest(rows), 'inputs': rows,
             'rustc': subprocess.check_output(['rustc', '--version'], cwd=ROOT, text=True).strip()}
    (OUT / 'native-inputs.json').write_text(json.dumps(guard, indent=2) + '\n')
    result = {'schema': 'rational-filled-cap-native-qualification-v1', 'qualifiedCodeRevision': revision,
              'lowerCheckpointRevision': LOWER, 'nativeCheckpointPassed': False, 'sourceGuardVerified': False,
              'inputsSha256': guard['inputsSha256'], 'checks': [], 'reusedLowerChecks': 0,
              'publicWasmRebuilt': False, 'freshCiQualified': False, 'semanticG1MatrixQualified': False,
              'independentStepQualified': False, 'browserQualified': False,
              'idealSourceSmoothnessQualified': False, 'physicalCapSmoothnessQualified': False, 'nativeSignedStepMatrixQualified': False,
              'fullGoalComplete': False}

    def save():
        (OUT / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')

    save()
    if sys.argv[1:]:
        raise ValueError('Changed native geometry requires fresh checks; no reuse option')
    for name, command in CHECKS:
        print('START', name, flush=True)
        start = time.time()
        log = OUT / 'logs' / f'{name}-final.log'
        with log.open('w') as f:
            run = subprocess.run(command, cwd=ROOT, stdout=f, stderr=subprocess.STDOUT)
        counts = [tuple(map(int, m)) for m in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', log.read_text())]
        result['checks'].append({'name': name, 'command': command, 'log': str(log.relative_to(OUT)),
                                 'exitCode': run.returncode, 'elapsedSeconds': round(time.time() - start, 3),
                                 'testResults': [{'passed': a, 'failed': b, 'ignored': c} for a, b, c in counts]})
        save()
        print('DONE', name, 'exit', run.returncode, 'tests', counts, flush=True)
        if run.returncode:
            return run.returncode
    result['sourceGuardVerified'] = rows == inputs()
    result['nativeCheckpointPassed'] = result['sourceGuardVerified']
    result['idealSourceSmoothnessQualified'] = result['nativeCheckpointPassed']
    result['nativeSignedStepMatrixQualified'] = result['nativeCheckpointPassed']
    result['physicalCapSmoothnessQualified'] = result['nativeCheckpointPassed']
    result['physicalCapSmoothnessScope'] = 'typed-native-original-rational-frame-planar-nonplanar-and-moving-affine-annular-cap-fixtures'
    result['newCapJsonWasmRuntimeQualified'] = False
    result['newCapStepQualified'] = False
    result['idealSourceSmoothnessScope'] = 'qualified-original-closed-material-and-rational-annular-cap-fixtures'
    result['totalPassed'] = sum(r['passed'] for c in result['checks'] for r in c['testResults'])
    result['totalIgnored'] = sum(r['ignored'] for c in result['checks'] for r in c['testResults'])
    save()
    print('SOURCE GUARD', result['sourceGuardVerified'], 'TOTAL', result['totalPassed'], 'ignored', result['totalIgnored'], flush=True)
    return 0 if result['sourceGuardVerified'] else 2


if __name__ == '__main__':
    raise SystemExit(main())
