import hashlib,json,os,subprocess,time,shutil
from pathlib import Path
root=Path('/private/tmp/osv-guided-geometric-seams-20261016')
q=root/'docs/qualification/guided-geometric-source-2026-10-16'
g=json.loads((q/'inputs.json').read_text())
def guard():return all(hashlib.sha256((root/i['path']).read_bytes()).hexdigest()==i['sha256'] for i in g['inputs'])
assert guard()
env=dict(os.environ,CARGO_BUILD_JOBS='2',RUST_TEST_THREADS='2',CARGO_TARGET_DIR='/private/tmp/osv-guided-geometric-seams-target-20261016')
cmd=['cargo','+nightly-2026-09-16','build','--locked','--manifest-path','crates/Cargo.toml','-p','geometry-wasm','--target','wasm32-unknown-unknown','--release']
r=subprocess.run(cmd,cwd=root,env=env);assert r.returncode==0 and guard()
artifact=Path('/private/tmp/osv-guided-geometric-private-artifact-20261016/geometry_wasm.wasm');artifact.parent.mkdir(exist_ok=True)
shutil.copy2(Path(env['CARGO_TARGET_DIR'])/'wasm32-unknown-unknown/release/geometry_wasm.wasm',artifact)
report=dict(schema='guided-geometric-private-wasm-build/1',sourceRevision=g['revision'],inputsSha256=g['inputsSha256'],command=cmd,wasmSha256=hashlib.sha256(artifact.read_bytes()).hexdigest(),wasmBytes=artifact.stat().st_size,artifact=str(artifact),buildExitCode=0,sourceGuardVerified=guard(),runtimeQualified=False,publicPackageQualified=False,fullGoalComplete=False)
out=Path('/private/tmp/osv-guided-geometric-wasm-build-20261016.json');out.write_text(json.dumps(report,indent=2)+'\n')
request=Path('/private/tmp/osv-guided-geometric-remaining-kernel-20261016/guided-frame-step/request.json')
while not request.exists():
    summary=Path('/private/tmp/osv-guided-geometric-remaining-kernel-20261016/summary.json')
    if summary.exists():
        state=json.loads(summary.read_text())
        assert not any(c['exitCode'] for c in state.get('checks',[])), 'Native qualifier failed before publishing the Rust input'
    assert guard(), 'Frozen source changed while awaiting the Rust-generated input'
    time.sleep(5)
r=subprocess.run(['node','--import','tsx','docs/qualification/guided-geometric-source-2026-10-16/wasm-runtime.mts',str(artifact),'/private/tmp/osv-guided-geometric-remaining-kernel-20261016/guided-frame-step/request.json','/private/tmp/osv-guided-geometric-wasm-runtime-20261016.json'],cwd=root,env=env)
report.update(runtimeQualified=r.returncode==0,sourceGuardVerified=guard());out.write_text(json.dumps(report,indent=2)+'\n');assert r.returncode==0 and guard()
