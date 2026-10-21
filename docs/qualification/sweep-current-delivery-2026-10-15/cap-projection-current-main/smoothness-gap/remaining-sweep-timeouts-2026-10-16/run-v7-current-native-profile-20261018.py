from pathlib import Path
import os,time,json,subprocess,hashlib
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-boolean-repair-v7-source-20261016');f=out/'v7-current-native-profile-20261018-receipt.json';assert not f.exists();sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();ledger=json.loads((stage/'main-native-source-sha256.json').read_text());runtime=json.loads((stage/'runtime-source-sha256.json').read_text());r={'state':'waiting-for-quiet-resources','passed':False,'pid':os.getpid(),'nativeLedgerSha256':sha(stage/'main-native-source-sha256.json'),'runtimeLedgerSha256':sha(stage/'runtime-source-sha256.json'),'fullGoalComplete':False,'scope':'Current-source native symbol profiling plus independent provider baseline; not final timeout/STEP/UI qualification.'}
def save():f.write_text(json.dumps(r,indent=2)+'\n')
def quiet():
 since=None
 while True:
  now=time.monotonic();load=os.getloadavg()
  if load[0]<=8:
   since=now if since is None else since
   if now-since>=30:return
  else:since=None
  r['lastObservedLoad']=list(load);save();time.sleep(5)
def identity():
 assert all(sha(stage/n)==h for n,h in ledger.items());assert all(sha(stage/n)==h for n,h in runtime.items())
save()
try:
 quiet();identity();r['state']='building-profiler';save()
 env={**os.environ,'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','RUSTUP_TOOLCHAIN':'nightly-2026-09-16','CARGO_NET_OFFLINE':'true'}
 with (out/'v7-current-native-profiler-build-20261018.log').open('w') as log:subprocess.run(['nice','-n','19','cargo','build','--locked','--offline','--release','--manifest-path','crates/Cargo.toml','-p','geometry-bridge','--example','bench_sweep_volume_request'],cwd=stage,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
 identity();binary=stage/'crates/target/release/examples/bench_sweep_volume_request';request=out/'v7-periodic-affine-hollow-trace-20261018.json.27.request.json';r.update(state='profiling',binarySha256=sha(binary),requestSha256=sha(request));save()
 with (out/'v7-current-native-volume-20261018.log').open('w') as log:
  p=subprocess.Popen([str(binary),str(request),str(out/'v7-current-native-volume-20261018.json')],stdout=log,stderr=subprocess.STDOUT);r['nativePid']=p.pid;save();time.sleep(1)
  sampled=subprocess.run(['/usr/bin/sample',str(p.pid),'3','1','-file',str(out/'v7-current-native-volume-20261018.sample.txt')],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);r['sampleReturncode']=sampled.returncode
  assert p.wait()==0
 identity();r.update(state='waiting-for-quiet-provider-baseline');save();quiet()
 files=sorted(set(json.loads((out/'v5-provider-baseline-inventory.json').read_text())['files']+['tests/runtimeGeometryActualBinding.test.ts']));node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node';report=out/'v7-provider-baseline-20261018.json';r.update(state='provider-baseline',testFiles=files);save()
 env={**env,'PATH':str(Path(node).parent)+os.pathsep+os.environ['PATH'],'VITEST_MAX_WORKERS':'1'};env.pop('NODE_OPTIONS',None)
 with (out/'v7-provider-baseline-20261018.log').open('w') as log:result=subprocess.run([node,'node_modules/vitest/vitest.mjs','run',*files,'--no-file-parallelism','--reporter=default','--reporter=json','--outputFile='+str(report)],cwd=stage,env=env,stdout=log,stderr=subprocess.STDOUT)
 identity();j=json.loads(report.read_text());r.update(providerTests=j['numTotalTests'],providerPassed=j['numPassedTests'],providerFailed=j['numFailedTests']);assert result.returncode==0 and j['numTotalTests']==j['numPassedTests'] and j['numFailedTests']==j['numPendingTests']==0
 r.update(state='complete',passed=True);save()
except Exception as e:r.update(state='failed',reason=str(e));save();raise
