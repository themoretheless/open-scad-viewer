from pathlib import Path
import hashlib,json,os,re,subprocess
OUT=Path(__file__).resolve().parent;ROOT=OUT.parents[2]
STAGE=Path('/private/tmp/osv-sweep-auto-map-global-20261030-r1')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
record=json.loads((OUT/'qualification-r1.json').read_text());assert record['state']=='complete' and record['passed']
ledger=json.loads((OUT/'source-r1.json').read_text());owned=json.loads((OUT/'owned-r1.json').read_text())
assert all(sha(STAGE/p)==h for p,h in ledger.items());assert all(sha(ROOT/p)==h for p,h in owned.items())
env={**os.environ,'CARGO_TARGET_DIR':'/private/tmp/osv-sweep-auto-qualified-20261030-r1','CARGO_BUILD_JOBS':'2'}
def save():(OUT/'qualification-r1.json').write_text(json.dumps(record,indent=2)+'\n')
for name,args,expected in [
 ('native-rational-owner',['-p','geometry-bridge','--test','rational_closed_material_certificate'],4),
 ('original-affine-volume-family',['-p','nurbs-core','--lib','analysis::affine_volume::'],None),
]:
 assert all(p['name']!=name for p in record['phases'])
 assert all(sha(STAGE/p)==h for p,h in ledger.items());assert all(sha(ROOT/p)==h for p,h in owned.items())
 argv=['cargo','test','--manifest-path','crates/Cargo.toml',*args,'--locked','--offline','--','--test-threads=2']
 phase={'name':name,'argv':argv,'cwd':str(STAGE),'state':'running'}
 record['phases'].append(phase);record.update(state='running',passed=False,extraDriverSha256=sha(__file__))
 save();log=OUT/('r1-'+name+'.log')
 with log.open('w')as f:run=subprocess.run(argv,cwd=STAGE,env=env,stdout=f,stderr=subprocess.STDOUT)
 counts=[int(n)for n in re.findall(r'test result: ok\. (\d+) passed; 0 failed',log.read_text())]
 phase.update(returncode=run.returncode,logSha256=sha(log),testsPassed=sum(counts))
 ok=run.returncode==0 and len(counts)==1 and counts[0]>0 and (expected is None or counts==[expected])
 phase['state']='passed' if ok else 'failed'
 record.update(state='running' if ok else 'failed',passed=False,testsPassed=sum(p.get('testsPassed',0)for p in record['phases']))
 save();assert ok,log
 print(name, sum(counts), 'passed',flush=True)
assert all(sha(STAGE/p)==h for p,h in ledger.items());assert all(sha(ROOT/p)==h for p,h in owned.items())
record.update(state='complete',passed=True);save()
