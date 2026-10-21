from pathlib import Path
import hashlib,json,os,subprocess,time
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-frozen-current-v4-source-20261016');main=Path.cwd();node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();native=json.loads((stage/'main-native-source-sha256.json').read_text());f=out/'frozen-current-v4-build.json';assert not f.exists();r={'state':'running','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'sourceRoot':str(stage),'stage':str(stage),'sourceFiles':len(native),'nativeLedgerSha256':sha(stage/'main-native-source-sha256.json'),'mainWasmBefore':sha(main/'public/wasm/geometry-kernel.wasm'),'mainWasmModified':False,'fullGoalComplete':False}
def save():
 p=f.with_suffix('.writing');p.write_text(json.dumps(r,indent=2)+'\n');p.replace(f)
for name,digest in native.items():assert sha(stage/name)==digest,name
save();start=time.monotonic()
with (out/'frozen-current-v4-build.log').open('w') as log:result=subprocess.run([node,'scripts/build-geometry-kernels.mjs'],cwd=stage,env={**os.environ,'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','CARGO_NET_OFFLINE':'true','RUSTUP_TOOLCHAIN':'nightly-2026-09-16','BINARYEN_CORES':'2','OSV_WASM_OPT_NATIVE':'/Users/themoretheless/.cargo/bin/wasm-opt'},stdout=log,stderr=subprocess.STDOUT)
changed=[name for name,digest in native.items() if sha(stage/name)!=digest];r.update(state='complete' if result.returncode==0 and not changed else 'failed',passed=result.returncode==0 and not changed,returncode=result.returncode,seconds=time.monotonic()-start,sourceChangedDuringBuild=changed,mainWasmAfter=sha(main/'public/wasm/geometry-kernel.wasm'))
if result.returncode==0:r.update(packedSha256=sha(stage/'public/wasm/geometry-kernel.wasm'),rawSha256=sha(stage/'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'))
save();print(json.dumps(r),flush=True);raise SystemExit(0 if r['passed'] else 1)
