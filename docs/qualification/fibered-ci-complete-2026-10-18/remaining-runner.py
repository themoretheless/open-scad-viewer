from pathlib import Path
import hashlib,json,os,re,subprocess,time
ROOT=Path('/private/tmp/osv-fibered-ci-architecture-20261016')
LOG=Path('/private/tmp/osv-fibered-ci-opt1-remaining-unsandboxed-20261017.log')
OUT=Path('/private/tmp/osv-fibered-ci-opt1-remaining-unsandboxed-20261017.json')
source=json.loads((ROOT/'docs/qualification/own-rust-cad-v75.json').read_text())
def bundle_hash():return hashlib.sha256(b''.join(p.encode()+b'\0'+(ROOT/p).read_bytes()+b'\n' for p in source['sourceBundle']['paths'])).hexdigest()
assert bundle_hash()==source['sourceBundle']['sha256']
cmd=['cargo','test','--locked','--manifest-path','crates/Cargo.toml','--workspace','--','--skip','aag']
env=dict(os.environ,CARGO_BUILD_JOBS='2',RUST_TEST_THREADS='2',CARGO_PROFILE_TEST_OPT_LEVEL='1',CARGO_TARGET_DIR=str(ROOT/'crates/target'))
start=time.time()
with LOG.open('w') as f:r=subprocess.run(cmd,cwd=ROOT,env=env,stdout=f,stderr=subprocess.STDOUT)
counts=[dict(passed=int(a),failed=int(b),ignored=int(c)) for a,b,c in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',LOG.read_text())]
report=dict(schema='native-test-profile-one-complete-remaining/1',revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),productionRustRevision='cec378bb23e029fe8498b6c89fe1e1d3e8aa86c7',sourceBundleSha256=source['sourceBundle']['sha256'],sourceGuardVerified=bundle_hash()==source['sourceBundle']['sha256'],command=cmd,environment={k:env[k] for k in ['CARGO_BUILD_JOBS','RUST_TEST_THREADS','CARGO_PROFILE_TEST_OPT_LEVEL']},sandbox='host-access: native Metal device is required by existing MLX cases',exitCode=r.returncode,elapsedSeconds=round(time.time()-start,3),totals={k:sum(c[k] for c in counts) for k in ['passed','failed','ignored']},testSummaryCount=len(counts),logSha256=hashlib.sha256(LOG.read_bytes()).hexdigest(),precedingAttempt='Sandbox denied Metal: 2049 passed, 2 failed, 3 ignored before abort. No success or complete-workspace credit for that attempt.',qualificationClaim='none',semanticG1MatrixQualified=False,fullGoalComplete=False)
OUT.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report),flush=True)
raise SystemExit(r.returncode)
