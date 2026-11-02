"""Complete every remaining audit gate while preserving the failed BRep gate."""
from pathlib import Path
import hashlib,json,os,re,subprocess,tarfile
Q=Path(__file__).resolve().parent;ROOT=Q.parents[2];STAGE=Path('/private/tmp/osv-sweep-gram-20261102-r4')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
ledger=json.loads((Q/'source-r4.json').read_text());changed=json.loads((Q/'changed-r4.json').read_text())
r=json.loads((Q/'qualification-r4.json').read_text());assert r['state']=='failed' and r['error']=='brep-sweep';assert [p['name']for p in r['phases']]==['topology','predicates','continuity','world-traces','brep-sweep']
for p in r['phases']:
 log=Q/('r4-'+p['name']+'.log');assert sha(log)==p['logSha256']
 counts=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed',log.read_text());p['testsPassed']=sum(int(a)for a,b in counts);p['testsFailed']=sum(int(b)for a,b in counts)
r.update(state='running',passed=False,auditComplete=False);r.pop('error',None)
def save():(Q/'audit-r4.json').write_text(json.dumps(r,indent=2)+'\n')
def guard():
 assert all(sha(STAGE/n)==h for n,h in ledger.items()),'Frozen source changed'
 assert all(sha(ROOT/n)==h for n,h in changed.items()),'Owned main source changed'
env={**os.environ,'CARGO_TARGET_DIR':'/private/tmp/osv-sweep-gram-target-20261102','CARGO_BUILD_JOBS':'2','CARGO_PROFILE_TEST_OPT_LEVEL':'1','CARGO_PROFILE_TEST_DEBUG_ASSERTIONS':'true','CARGO_PROFILE_TEST_OVERFLOW_CHECKS':'true','OSV_GRAM_BODY_DIR':str(Q/'native-body'),'OSV_AFFINE_FRAME_STEP_DIR':str(Q/'native-affine'),'OSV_GENERAL_CERTIFICATE_STEP':str(Q/'general-capped.step')}
phases=[('native',['test','-p','geometry-bridge','--test','authored_seam_map','--test','gram_algebraic_body','--test','bspline_boundary_joins','--test','complete_multispan_seams','--test','rational_closed_material_certificate','--test','general_boundary_certificate','--test','affine_frame_source_certificate','--test','arc_bishop_source_certificate']),('no-codec',['check','-p','cad-predicates','-p','nurbs-core','--no-default-features'])]
save()
try:
 for name,args in phases:
  guard();argv=['cargo',*args,'--manifest-path','crates/Cargo.toml','--locked','--offline']
  if args[0]=='test':argv+=['--','--test-threads=2']
  print(name+': started',flush=True);log=Q/('r4-'+name+'.log')
  with log.open('w')as f:done=subprocess.run(argv,cwd=STAGE,env=env,stdout=f,stderr=subprocess.STDOUT)
  counts=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed',log.read_text())
  r['phases'].append({'name':name,'argv':argv,'returncode':done.returncode,'testsPassed':sum(int(a)for a,b in counts),'testsFailed':sum(int(b)for a,b in counts),'logSha256':sha(log)})
  save();print(name+': return '+str(done.returncode),flush=True)
 guard()
 with tarfile.open(Q/'audited-source-r4.tar.gz','w:gz',compresslevel=3)as t:
  for n in ledger:t.add(STAGE/n,arcname=n,recursive=False)
 r.update(state='complete',auditComplete=True,passed=all(p['returncode']==0 for p in r['phases']),sourceGuardVerified=True,changedMainMatches=True,testsPassed=sum(p['testsPassed']for p in r['phases']),testsFailed=sum(p['testsFailed']for p in r['phases']),sourceArchiveSha256=sha(Q/'audited-source-r4.tar.gz'));save()
except BaseException as e:r.update(state='failed',error=str(e));save();raise
