#!/usr/bin/env python3
"""Bind completed native, independent STEP and private runtime receipts."""
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[2]
EXPLORATORY=Path('/private/tmp/osv-source-fibered-step-inspect-20261016')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def read(p):return json.loads(p.read_text())
def write(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
def main():
    spec=importlib.util.spec_from_file_location('frozen_guard',OUT/'validate.py')
    guard_module=importlib.util.module_from_spec(spec);spec.loader.exec_module(guard_module)
    guard=read(OUT/'native-inputs.json');summary=read(OUT/'summary.json')
    assert summary['nativeCheckpointPassed'] and summary['sourceGuardVerified']
    assert len(summary['checks'])==len(guard_module.CHECKS)==15
    assert all(c['exitCode']==0 and sha(OUT/c['log'])==c['logSha256'] for c in summary['checks'])
    assert guard['inputs']==guard_module.inputs()
    for name,count in [('fibered-occt-report.json',4),('sphere-occt-report.json',4),('rational-cap-occt-report.json',6)]:
        report=read(OUT/name);assert report['passed'] and len(report['cases'])==count
        assert all(c['passed'] for c in report['cases'])
    build=read(OUT/'wasm-build.json');runtime=read(OUT/'wasm-runtime-report.json')
    artifact=Path(build['preservedArtifact'])
    assert build['buildExitCode']==0 and build['sourceRevision']==guard['qualifiedCodeRevision']
    assert build['nativeInputsSha256']==guard['inputsSha256']
    assert sha(artifact)==build['wasmSha256']==runtime['wasmSha256']
    assert artifact.stat().st_size==build['wasmBytes']==runtime['wasmBytes']
    assert runtime['passed'] and len(runtime['cases'])==4 and runtime['strictG2RefusedTrueG1'] and runtime['explicitC0Preserved']
    assert all(c['passed'] and c['wasmStepReaudited'] and c['shortBudgetRefused'] and c['wrongModelRefused'] for c in runtime['cases'])
    old_receipt=read(ROOT/'docs/qualification/implicit-sphere-test-contract-2026-10-16/wasm-runtime-inputs.json')
    rows=[]
    previous_runtime=[]
    for folder,report_name,harness,count in [
        ('sphere-step','sphere-wasm-runtime-report.json','docs/qualification/implicit-sphere-source-2026-10-16/wasm-runtime.mts',4),
        ('rational-cap-step','rational-cap-wasm-runtime-report.json','docs/qualification/rational-cap-source-audit-2026-10-16/wasm-runtime.mts',6),
    ]:
        report=read(OUT/report_name)
        assert report['passed'] and len(report['cases'])==count
        assert report['wasmSha256']==build['wasmSha256'] and report['wasmBytes']==build['wasmBytes']
        assert all(c['passed'] for c in report['cases'])
        if count==6:assert report['sharpC0Passed']
        original=ROOT/'docs/qualification/implicit-sphere-test-contract-2026-10-16'/folder
        fresh_files=sorted((OUT/folder).glob('*.source.json'))
        assert len(fresh_files)==count
        for fresh in fresh_files:
            assert fresh.read_bytes()==(original/fresh.name).read_bytes()
            rows.append(dict(path=str(fresh.relative_to(ROOT)),sha256=sha(fresh)))
        before=next(r for r in old_receipt['inputs'] if r['path']==harness)
        assert sha(ROOT/harness)==before['sha256']
        rows.append(dict(path=harness,sha256=sha(ROOT/harness)))
        previous_runtime.append(dict(path=str((OUT/report_name).relative_to(ROOT)),sha256=sha(OUT/report_name),cases=count))
    for name in ['basic','oblique-ascending','oblique-descending','three-pieces']:
        fresh=OUT/'fibered-step'/name/'source.json';tested=EXPLORATORY/name/'source.json'
        assert fresh.read_bytes()==tested.read_bytes(),name
        rows.append(dict(path=str(fresh.relative_to(ROOT)),sha256=sha(fresh)))
    for p in [OUT/'wasm-runtime.mts',ROOT/'src/services/valueBinaryCodec.ts',Path(__file__).resolve()]:
        rows.append(dict(path=str(p.relative_to(ROOT)),sha256=sha(p)))
    rows.sort(key=lambda r:r['path'])
    for src,name in [('/private/tmp/osv-source-fibered-private-wasm-build-20261016.log','private-wasm-build.log'),('/private/tmp/osv-source-fibered-wasm-runtime-20261016.log','private-wasm-runtime.log'),('/private/tmp/osv-source-fibered-closed-regression-20261016.log','exploratory-closed-regression.log'),('/private/tmp/osv-source-fibered-sphere-wasm-runtime-20261016.log','sphere-wasm-runtime.log'),('/private/tmp/osv-source-fibered-rational-cap-wasm-runtime-20261016.log','rational-cap-wasm-runtime.log')]:
        shutil.copy2(src,OUT/'logs'/name)
    receipt=dict(schema='original-fibered-source-private-runtime-correspondence/1',qualifiedCodeRevision=guard['qualifiedCodeRevision'],nativeInputsSha256=guard['inputsSha256'],wasmSha256=build['wasmSha256'],wasmBytes=build['wasmBytes'],node=subprocess.check_output(['node','--version'],cwd=ROOT,text=True).strip(),tsx=read(ROOT/'node_modules/tsx/package.json')['version'],inputs=rows,inputsSha256=guard_module.digest(rows),inputCorrespondenceVerified=True,buildLogSha256=sha(OUT/'logs/private-wasm-build.log'),runtimeLogSha256=sha(OUT/'logs/private-wasm-runtime.log'),runtimeReportSha256=sha(OUT/'wasm-runtime-report.json'),previousSourceRuntimeReports=previous_runtime,runtimeCases=14,publicPackageQualified=False,browserQualified=False,semanticG1MatrixQualified=False,fullGoalComplete=False)
    write(OUT/'wasm-runtime-inputs.json',receipt)
    summary.update(privateReleaseWasmRuntimeQualified=True,privateWasmSha256=build['wasmSha256'],privateWasmBytes=build['wasmBytes'],privateWasmRuntimeInputCorrespondenceVerified=True,privateWasmRuntimeCases=14,previousSourceClassesWasmRequalified=True)
    write(OUT/'summary.json',summary)
    print(json.dumps({'nativePassed':summary['totals'],'privateWasmRuntimePassed':True,'wasmSha256':build['wasmSha256'],'fullGoalComplete':False}))
if __name__=='__main__':main()
