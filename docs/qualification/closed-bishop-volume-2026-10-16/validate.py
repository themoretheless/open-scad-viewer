#!/usr/bin/env python3
"""Frozen native qualification; no WASM/CI/browser credit is inferred."""
import hashlib
import json
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]
OUT=Path(__file__).resolve().parent
BASELINE='56aff865de63a22015d2205eb00447f7df5261a3'
BASE=['cargo','test','--locked','--manifest-path','crates/Cargo.toml']
CHECKS=[
    ('nurbs-full',BASE+['-p','nurbs-core','--lib']),
    ('source-volume-matrices',BASE+['-p','nurbs-core','--test','ideal_volume','--test','ideal_spatial_volume','--test','ideal_volume_atlas','--test','ideal_closed_spatial_volume','--','--nocapture']),
    ('brep-full',BASE+['-p','brep-core','--lib','--','--skip','aag']),
    ('brep-aag-serial',BASE+['-p','brep-core','--lib','aag','--','--test-threads=1']),
    ('bridge-full',BASE+['-p','geometry-bridge','--lib']),
    ('native-json-step',BASE+['-p','geometry-bridge','--test','general_boundary_certificate','--','--nocapture']),
    ('no-codec-libraries',['cargo','check','--locked','--manifest-path','crates/Cargo.toml','-p','brep-core','-p','nurbs-intersect','-p','brep-intersect','--no-default-features']),
]
def inputs():
    files=subprocess.check_output(['git','-c','core.fsmonitor=false','ls-files','--cached','--others','--exclude-standard'],cwd=ROOT,text=True).splitlines()
    return [{'path':p,'sha256':hashlib.sha256((ROOT/p).read_bytes()).hexdigest()} for p in sorted(set(files)) if p.startswith('crates/') or p=='rust-toolchain.toml']
def digest(rows):
    return hashlib.sha256(json.dumps(rows,sort_keys=True,separators=(',',':')).encode()).hexdigest()

def main():
    rows=inputs()
    guard={'schema':'closed-bishop-volume-native-inputs-v1','baselineRevision':BASELINE,
        'digestEncoding':'sha256 of UTF-8 compact sort_keys JSON of sorted inputs list',
        'inputCount':len(rows),'inputsSha256':digest(rows),'inputs':rows,
        'rustc':subprocess.check_output(['rustc','--version'],cwd=ROOT,text=True).strip()}
    (OUT/'native-inputs.json').write_text(json.dumps(guard,indent=2)+'\n')
    result={'schema':'closed-bishop-volume-native-qualification-v1','baselineRevision':BASELINE,
        'nativeCheckpointPassed':False,'sourceGuardVerified':False,'inputsSha256':guard['inputsSha256'],
        'checks':[],'publicWasmRebuilt':False,'freshCiQualified':False,'semanticG1MatrixQualified':False,
        'independentStepQualified':False,'browserQualified':False,'fullGoalComplete':False}
    def save(): (OUT/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
    save()  # Never leave a stale success marker after an invalid resume request.
    if sys.argv[1:]:
        raise ValueError("This geometry successor requires fresh checks; no reuse option")
    checks=CHECKS
    for name,command in checks:
        print('START',name,flush=True)
        start=time.time()
        log=OUT/'logs'/f'{name}-final.log'
        with log.open('w') as f: run=subprocess.run(command,cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
        content=log.read_text()
        counts=[tuple(map(int,m)) for m in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',content)]
        item={'name':name,'command':command,'log':str(log.relative_to(OUT)),
            'exitCode':run.returncode,'elapsedSeconds':round(time.time()-start,3),
            'testResults':[{'passed':a,'failed':b,'ignored':c} for a,b,c in counts]}
        result['checks'].append(item);save()
        print('DONE',name,'exit',run.returncode,'tests',counts,flush=True)
        if run.returncode: return run.returncode
    final=inputs()
    result['sourceGuardVerified']=rows==final
    result['nativeCheckpointPassed']=result['sourceGuardVerified']
    result['totalPassed']=sum(r['passed'] for c in result['checks'] for r in c['testResults'])
    result['totalIgnored']=sum(r['ignored'] for c in result['checks'] for r in c['testResults'])
    save()
    print('SOURCE GUARD',result['sourceGuardVerified'],'TOTAL',result['totalPassed'],'ignored',result['totalIgnored'],flush=True)
    return 0 if result['sourceGuardVerified'] else 2
if __name__=='__main__':raise SystemExit(main())
