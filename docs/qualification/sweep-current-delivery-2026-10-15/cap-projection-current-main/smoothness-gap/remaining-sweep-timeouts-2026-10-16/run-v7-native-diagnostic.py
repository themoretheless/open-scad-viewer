from pathlib import Path
import json,os,subprocess,time,hashlib
out=Path(__file__).resolve().parent
stage=Path('/private/tmp/osv-sweep-boolean-repair-v7-source-20261016')
f=out/'v7-native-diagnostic-receipt.json'
assert not f.exists()
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
ledger=stage/'main-native-source-sha256.json'
binding=json.loads(ledger.read_text())
r={'state':'waiting-for-quiet-resources','passed':False,'pid':os.getpid(),'stage':str(stage),'nativeLedgerSha256':sha(ledger),'resourceGate':{'maxOneMinuteLoad':8,'stableSeconds':30,'compilerNice':19},'published':False,'fullGoalComplete':False}
def save():f.write_text(json.dumps(r,indent=2)+'\n')
since=None
while True:
 load=os.getloadavg();now=time.monotonic()
 if load[0]<=8:
  since=now if since is None else since
  if now-since>=30:break
 else:since=None
 r.update(lastObservedLoad=list(load));save();time.sleep(5)
assert sha(ledger)==r['nativeLedgerSha256']
assert all(sha(stage/k)==v for k,v in binding.items())
argv=['nice','-n','19','cargo','test','--locked','--offline','--release','--manifest-path','crates/Cargo.toml','-p','geometry-bridge','--lib','rotated_boolean_failure_diagnostic','--','--nocapture']
r.update(state='running',argv=argv);save()
with (out/'v7-native-diagnostic.log').open('w') as log:
 result=subprocess.run(argv,cwd=stage,env={**os.environ,'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','RUSTUP_TOOLCHAIN':'nightly-2026-09-16','CARGO_NET_OFFLINE':'true'},stdout=log,stderr=subprocess.STDOUT)
r.update(state='complete' if result.returncode==0 else 'failed',passed=result.returncode==0,returncode=result.returncode,logSha256=sha(out/'v7-native-diagnostic.log'));save()
