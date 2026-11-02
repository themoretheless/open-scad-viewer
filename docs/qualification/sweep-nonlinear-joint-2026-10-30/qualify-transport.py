from pathlib import Path
import hashlib,json,os,re,subprocess,tarfile
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[2]
STAGE=Path('/private/tmp/osv-sweep-nonlinear-joint-20261030-r2')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
r=json.loads((OUT/'qualification.json').read_text())
assert r['passed'] and r['state']=='complete'
ledger=json.loads((OUT/'source-r2.json').read_text())
assert all(sha(STAGE/n)==h for n,h in ledger.items())
overlay=json.loads((OUT/'native-overlay.json').read_text())
for n,v in overlay.items():
 p=OUT/v['file'];assert sha(p)==sha(ROOT/n)==v['sha256'];(STAGE/n).write_bytes(p.read_bytes());ledger[n]=v['sha256']
(OUT/'source-r3.json').write_text(json.dumps(ledger,indent=2)+'\n')
record={'state':'running','passed':False,'sourceIndexSha256':sha(OUT/'source-r3.json'),'phases':[],
 'productionCodeUnchangedFromR2':True,'limitsIncreased':False,'headed':False,'wasmPublished':False}
def save():(OUT/'transport-qualification.json').write_text(json.dumps(record,indent=2)+'\n')
assert not (OUT/'transport-qualification.json').exists();save()
env={**os.environ,'CARGO_TARGET_DIR':'/private/tmp/osv-sweep-remaining-20261028','CARGO_BUILD_JOBS':'2'}
try:
 for test in ['affine_frame_source_certificate','arc_bishop_source_certificate']:
  argv=['cargo','test','--manifest-path','crates/Cargo.toml','-p','geometry-bridge','--test',test,'--locked','--offline','--','--test-threads=2']
  phase={'name':test,'argv':argv,'state':'running'};record['phases'].append(phase);save();print(test+': started',flush=True)
  log=OUT/('joint-'+test+'.log')
  with log.open('w')as f:p=subprocess.run(argv,cwd=STAGE,env=env,stdout=f,stderr=subprocess.STDOUT)
  counts=[int(n)for n in re.findall(r'test result: ok\. (\d+) passed; 0 failed',log.read_text())]
  phase.update(returncode=p.returncode,logSha256=sha(log),testsPassed=sum(counts))
  assert p.returncode==0 and counts and min(counts)>0,test
  phase['state']='passed';save();print(test+': passed',flush=True)
 assert all(sha(STAGE/n)==h for n,h in ledger.items())
 assert all(sha(ROOT/n)==v['sha256'] for n,v in overlay.items())
 with tarfile.open(OUT/'qualified-source-r3.tar.gz','w:gz',compresslevel=3)as tar:
  for n in ledger:tar.add(STAGE/n,arcname=n,recursive=False)
 record.update(state='complete',passed=True,sourceGuardVerified=True,testsPassed=sum(p['testsPassed']for p in record['phases']),sourceArchiveSha256=sha(OUT/'qualified-source-r3.tar.gz'));save()
except BaseException as e:
 record.update(state='failed',error=str(e));save();raise
