"""Finish after a new integration-test-only borrow repair; no core phase rerun."""
from pathlib import Path
import hashlib,json,os,re,shutil,subprocess,tarfile
Q=Path(__file__).resolve().parent;ROOT=Q.parents[2];STAGE=Path('/private/tmp/osv-sweep-gram-20261102-r2')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
old=json.loads((Q/'qualification-r2.json').read_text());ledger=json.loads((Q/'source-r2.json').read_text());changed=json.loads((Q/'changed-r2.json').read_text())
expected=['predicates','continuity','world-traces','brep-sweep']
assert [p['name']for p in old['phases']]==expected and all(p['returncode']==0 for p in old['phases'])
assert old['state']=='failed' and old['error']=='Owned main source changed'
assert all(sha(STAGE/n)==h for n,h in ledger.items())
for p in old['phases']:assert sha(Q/(p['name']+'.log'))==p['logSha256']
native='crates/geometry-bridge/tests/gram_algebraic_body.rs'
assert all(n==native or sha(ROOT/n)==h for n,h in changed.items())
assert sha(ROOT/native)!=changed[native]
# These passed crate unit phases do NOT depend on geometry-bridge integration
# tests. Preserve their exact source evidence; repair only this new test file.
assert native.startswith('crates/geometry-bridge/tests/')
assert (STAGE/native).read_text().replace('for coedge in &mut model.loops[outer].coedges {','for coedge in &mut model.0.loops[outer].coedges {').replace('let mut pcurve = model.edges[coedge.edge].curve.clone();','let mut pcurve = model.0.edges[coedge.edge].curve.clone();')==(ROOT/native).read_text()
with tarfile.open(Q/'pre-native-test-repair-source.tar.gz','w:gz',compresslevel=3)as t:
 for n in ledger:t.add(STAGE/n,arcname=n,recursive=False)
shutil.copy2(ROOT/native,STAGE/native);ledger[native]=sha(STAGE/native);changed[native]=sha(ROOT/native)
(Q/'source-r3.json').write_text(json.dumps(ledger,indent=2)+'\n');(Q/'changed-r3.json').write_text(json.dumps(changed,indent=2)+'\n')
r={**old,'state':'running','passed':False,'phases':old['phases'].copy(),'testOnlySourceRepair':[native],'priorPhaseSourceIndexSha256':sha(Q/'source-r2.json'),'sourceIndexSha256':sha(Q/'source-r3.json'),'mainHeadAtQualificationStart':old['baseCommit']}
r.pop('baseCommit',None);r.pop('error',None)
def save():(Q/'qualification-r3.json').write_text(json.dumps(r,indent=2)+'\n')
def guard():
 assert all(sha(STAGE/n)==h for n,h in ledger.items()),'Frozen source changed'
 assert all(sha(ROOT/n)==h for n,h in changed.items()),'Owned main source changed'
env={**os.environ,'CARGO_TARGET_DIR':'/private/tmp/osv-sweep-gram-target-20261102','CARGO_BUILD_JOBS':'2','OSV_GRAM_BODY_DIR':str(Q/'native-body'),'OSV_AFFINE_FRAME_STEP_DIR':str(Q/'native-affine'),'OSV_GENERAL_CERTIFICATE_STEP':str(Q/'general-capped.step')}
phases=[('native',['test','-p','geometry-bridge','--test','authored_seam_map','--test','gram_algebraic_body','--test','bspline_boundary_joins','--test','complete_multispan_seams','--test','rational_closed_material_certificate','--test','general_boundary_certificate','--test','affine_frame_source_certificate','--test','arc_bishop_source_certificate']),('no-codec',['check','-p','cad-predicates','-p','nurbs-core','--no-default-features'])]
save()
try:
 for name,args in phases:
  guard();argv=['cargo',*args,'--manifest-path','crates/Cargo.toml','--locked','--offline']
  if args[0]=='test':argv+=['--','--test-threads=2']
  print(name+': started',flush=True)
  log=Q/(name+'.log')
  with log.open('w')as f:done=subprocess.run(argv,cwd=STAGE,env=env,stdout=f,stderr=subprocess.STDOUT)
  counts=re.findall(r'test result: ok\. (\d+) passed; 0 failed',log.read_text())
  r['phases'].append({'name':name,'argv':argv,'returncode':done.returncode,'testsPassed':sum(map(int,counts)),'logSha256':sha(log)})
  save();assert done.returncode==0 and (args[0]=='check' or counts),name
  print(name+': passed '+str(sum(map(int,counts))),flush=True)
 guard()
 with tarfile.open(Q/'qualified-source-r3.tar.gz','w:gz',compresslevel=3)as t:
  for n in ledger:t.add(STAGE/n,arcname=n,recursive=False)
 r.update(state='complete',passed=True,sourceGuardVerified=True,changedMainMatches=True,testsPassed=sum(p['testsPassed']for p in r['phases']),sourceArchiveSha256=sha(Q/'qualified-source-r3.tar.gz'));save()
except BaseException as e:r.update(state='failed',error=str(e));save();raise
