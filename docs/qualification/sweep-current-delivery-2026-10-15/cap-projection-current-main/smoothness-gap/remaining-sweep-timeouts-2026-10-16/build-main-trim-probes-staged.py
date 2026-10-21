"""Build actual main inputs into isolated assets; never overwrite peer runtime."""
import hashlib,json,os,subprocess,time
from pathlib import Path
root=Path.cwd();out=Path(__file__).resolve().parent
stage=Path('/private/tmp/osv-sweep-main-trim-probes-delivery-20261016')
assert not stage.exists(),'Preserve earlier staging'
stage.mkdir()
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def sources():
 files=[]
 for package in (root/'crates').iterdir():
  if not package.is_dir() or package.name=='target':continue
  if (package/'src').is_dir():files.extend(p for p in (package/'src').rglob('*') if p.is_file())
  files.extend(p for p in [package/'Cargo.toml',package/'build.rs'] if p.is_file())
 files.extend(root/x for x in ['crates/Cargo.toml','crates/Cargo.lock','rust-toolchain.toml','package-lock.json','scripts/build-geometry-kernels.mjs','scripts/build-wasm-brotli.mjs','scripts/reproducible-cargo.mjs','scripts/wasm-optimize.mjs','scripts/wasm-opt-cache.mjs','scripts/pack-geometry-kernel.mjs'])
 return {str(p.relative_to(root)):sha(p) for p in files}
before=sources();(stage/'main-native-source-sha256.json').write_text(json.dumps(before,indent=2)+'\n')
f=out/'main-trim-probes-staged-build.json';assert not f.exists()
r={'schema':'main-native-level-reuse-staged-build/1','state':'running','pid':os.getpid(),'driverSha256':sha(__file__),'stage':str(stage),'mainWasmBefore':sha(root/'public/wasm/geometry-kernel.wasm'),'sourceLedgerSha256':sha(stage/'main-native-source-sha256.json'),'sourceFiles':len(before),'mainWasmModified':False,'fullGoalComplete':False}
def save():
 q=f.with_suffix('.writing');q.write_text(json.dumps(r,indent=2)+'\n');q.replace(f)
save();start=time.monotonic()
env={**os.environ,'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','CARGO_NET_OFFLINE':'true','RUSTUP_TOOLCHAIN':'nightly-2026-09-16','BINARYEN_CORES':'2','OSV_WASM_OPT_NATIVE':'/Users/themoretheless/.cargo/bin/wasm-opt'}
with (out/'main-trim-probes-staged-build.log').open('w') as log:
 result=subprocess.run(['/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node','scripts/build-geometry-kernels.mjs','--staging-root='+str(stage)],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT)
after=sources();changed=[x for x in set(before)|set(after) if before.get(x)!=after.get(x)]
r.update(state='complete' if result.returncode==0 and not changed else 'failed',returncode=result.returncode,seconds=time.monotonic()-start,sourceChangedDuringBuild=changed,mainWasmAfter=sha(root/'public/wasm/geometry-kernel.wasm'))
if result.returncode==0:
 r.update(packedSha256=sha(stage/'public/wasm/geometry-kernel.wasm'),rawSha256=sha(root/'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'))
save();print(json.dumps(r),flush=True);raise SystemExit(0 if r['state']=='complete' else 1)
