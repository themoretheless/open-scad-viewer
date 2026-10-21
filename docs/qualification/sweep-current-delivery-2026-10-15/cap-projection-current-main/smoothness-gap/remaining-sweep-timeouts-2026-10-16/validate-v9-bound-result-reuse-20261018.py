"""Native bit-equivalence and alternating paired timing; no runtime publication."""
from pathlib import Path
import hashlib,json,subprocess,os,time,statistics
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-bound-result-reuse-v9-source-20261018');base=Path('/private/tmp/osv-sweep-boolean-repair-v7-source-20261016');f=out/'v9-bound-result-reuse-validation-20261018.json';assert not f.exists();sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();ledgerpath=stage/'main-native-source-sha256.json';ledger=json.loads(ledgerpath.read_text());baseline=base/'crates/target/release/examples/bench_sweep_volume_request';request=out/'v7-periodic-affine-hollow-trace-20261018.json.27.request.json';r={'state':'waiting-for-quiet-resources','passed':False,'pid':os.getpid(),'nativeLedgerSha256':sha(ledgerpath),'baselineBinarySha256':sha(baseline),'requestSha256':sha(request),'phases':[],'runs':[],'pairedWinValidated':False,'published':False,'fullGoalComplete':False}
def save():
 p=f.with_suffix('.writing');p.write_text(json.dumps(r,indent=2)+'\n');p.replace(f)
def identity():
 assert sha(ledgerpath)==r['nativeLedgerSha256'];assert all(sha(stage/n)==h for n,h in ledger.items());assert sha(baseline)==r['baselineBinarySha256'];assert sha(request)==r['requestSha256']
def quiet():
 since=None
 while True:
  now=time.monotonic();load=os.getloadavg()
  if load[0]<=8:
   since=now if since is None else since
   if now-since>=30:return
  else:since=None
  r['lastObservedLoad']=list(load);save();time.sleep(5)
def compile(name,argv):
 quiet();identity();r['state']=name;item={'name':name,'state':'running','argv':argv,'launchLoad':list(os.getloadavg())};r['phases'].append(item);save();start=time.monotonic()
 with (out/('v9-'+name+'-20261018.log')).open('w') as log:result=subprocess.run(['nice','-n','19',*argv],cwd=stage,env={**os.environ,'RUSTUP_TOOLCHAIN':'nightly-2026-09-16','CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','CARGO_NET_OFFLINE':'true'},stdout=log,stderr=subprocess.STDOUT)
 identity();item.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode,seconds=time.monotonic()-start);save();assert result.returncode==0,name+' failed'
save()
try:
 compile('control-reuse-native-tests',['cargo','test','--locked','--offline','--release','--manifest-path','crates/Cargo.toml','-p','nurbs-core','--lib','surface_contact','--','--nocapture'])
 compile('control-reuse-profiler-build',['cargo','build','--locked','--offline','--release','--manifest-path','crates/Cargo.toml','-p','geometry-bridge','--example','bench_sweep_volume_request'])
 candidate=stage/'crates/target/release/examples/bench_sweep_volume_request';r['candidateBinarySha256']=sha(candidate);assert r['candidateBinarySha256']!=r['baselineBinarySha256'];save();identity()
 reference=None
 def measure(label,tag,record):
  global reference
  binary=baseline if label=='baseline' else candidate;prefix=out/('v9-bound-result-reuse-'+tag+'-'+label+'-20261018.json');assert not prefix.exists();startload=list(os.getloadavg());since=time.monotonic()
  with prefix.with_suffix('.stdout.log').open('w') as log:result=subprocess.run([str(binary),str(request),str(prefix)],stdout=log,stderr=subprocess.STDOUT)
  assert result.returncode==0,(label,tag);j=json.loads(prefix.read_text());assert j['solidGeometryCertified'] is True;report=Path(str(prefix)+'.report.json');response=report.read_bytes()
  if reference is None:reference=response
  assert response==reference,'Native report changed: includes all geometric verdicts, work counters, orientations, nesting and retained charts'
  item={'label':label,'tag':tag,'milliseconds':j['milliseconds'],'wallSeconds':time.monotonic()-since,'steps':j['steps'],'reportSha256':sha(report),'startLoad':startload,'endLoad':list(os.getloadavg()),'scope':'Native only; not a WASM/browser/test deadline gate.'}
  if record:r['runs'].append(item);save()
 quiet();r['state']='warming-native-binaries';save();measure('baseline','warm',False);measure('candidate','warm',False)
 for pair,order in enumerate([['baseline','candidate'],['candidate','baseline'],['baseline','candidate'],['candidate','baseline']]):
  quiet();identity();r['state']='paired-native-measurements';save()
  for label in order:measure(label,str(pair),True)
 gains=[]
 for pair in range(4):
  rows={x['label']:x for x in r['runs'] if x['tag']==str(pair)};a=rows['baseline'];b=rows['candidate'];gains.append(1-b['milliseconds']/a['milliseconds'])
 r.update(pairedGains=gains,medianGain=statistics.median(gains),reportsBitIdentical=True)
 r['pairedWinValidated']=all(g>0 for g in gains) and r['medianGain']>=0.05
 r.update(state='complete' if r['pairedWinValidated'] else 'complete-no-consistent-win',passed=r['pairedWinValidated']);save()
except Exception as e:r.update(state='failed',passed=False,reason=str(e));save();raise
