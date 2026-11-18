from pathlib import Path
import os,json,hashlib,re,subprocess,sys
Q=Path(__file__).resolve().parent;ROOT=Q.parents[2];S=Path('/private/tmp/osv-quintic-global-20261103')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
ledger=json.loads((Q/'source.json').read_text());owned=json.loads((Q/'owned.json').read_text());inputs=json.loads((Q/'inputs.json').read_text())
def guard():
 assert all(sha(S/n)==h for n,h in ledger.items()),'Frozen source changed'
 assert all(sha(ROOT/n)==h for n,h in owned.items()),'Owned main source changed'
 assert all(sha(Q/n)==h for n,h in inputs.items()),'Source model changed'
env={**os.environ,'CARGO_TARGET_DIR':'/private/tmp/osv-sweep-gram-target-20261102','CARGO_BUILD_JOBS':'2','CARGO_PROFILE_TEST_OPT_LEVEL':'1','CARGO_PROFILE_TEST_DEBUG_ASSERTIONS':'true','CARGO_PROFILE_TEST_OVERFLOW_CHECKS':'true','CARGO_PROFILE_DEV_OPT_LEVEL':'1','CARGO_PROFILE_DEV_DEBUG_ASSERTIONS':'true','CARGO_PROFILE_DEV_OVERFLOW_CHECKS':'true','OSV_GRAM_BODY_DIR':str(Q/'original-step'),'OSV_AFFINE_FRAME_STEP_DIR':str(Q/'native-affine-matrix'),'OSV_GENERAL_CERTIFICATE_STEP':str(Q/'general-capped.step')}
def cargo(args,test=True):return ['cargo',*args,'--manifest-path',str(S/'crates/Cargo.toml'),'--locked','--offline']+(['--','--test-threads=2']if test else [])
phases=[('coordinates',cargo(['test','-p','nurbs-core','--test','affine_critical_coordinates'])),('critical-global',cargo(['test','-p','brep-core','--lib','critical_projection'])),('native-affine',['cargo','run','--manifest-path',str(Q/'probe/Cargo.toml'),'--locked','--offline','--',str(Q/'source-body.json'),str(Q/'affine-body')]),('exact-oracle',['python3',str(Q/'exact_oracle.py')]),('occt',[str(ROOT/'.venv-cad-step/bin/python'),str(Q/'verify-affine-occt.py'),str(Q/'affine-body')]),('nurbs-full',cargo(['test','-p','nurbs-core','--lib'])),('algebraic-maps',cargo(['test','-p','nurbs-core','--no-fail-fast','--test','original_quadric_carrier','--test','original_quadric_graph'])),('brep-full',cargo(['test','-p','brep-core','--lib'])),('native-matrix',cargo(['test','-p','geometry-bridge','--no-fail-fast','--test','authored_seam_map','--test','gram_algebraic_body','--test','bspline_boundary_joins','--test','complete_multispan_seams','--test','rational_closed_material_certificate','--test','general_boundary_certificate','--test','affine_frame_source_certificate','--test','arc_bishop_source_certificate','--test','original_quadric_graph'])),('no-codec',cargo(['check','-p','cad-predicates','-p','nurbs-core','-p','brep-core','--no-default-features'],False))]
r={'state':'running','passed':False,'fullGoalComplete':False,'wasmPublished':False,'uiChecked':False,'limitsRaised':False,'phases':[],'frozenFiles':len(ledger)}
def save():(Q/'audit.json').write_text(json.dumps(r,indent=2)+'\n')
if '--resume' in sys.argv:
 old=json.loads((Q/'audit.json').read_text())
 previous=[]
 for phase in old['phases']:
  if phase['returncode']!=0:break
  previous.append(phase)
 for p,(name,argv) in zip(previous,phases):
  assert p['name']==name and p['argv']==argv and sha(Q/(name+'.log'))==p['logSha256']
 r['phases']=previous
save()
try:
 for index,(name,argv) in enumerate(phases):
  if index<len(r['phases']):continue
  guard();print(name+': started',flush=True);log=Q/(name+'.log')
  with log.open('w')as f:done=subprocess.run(argv,cwd=S,env=env,stdout=f,stderr=subprocess.STDOUT)
  counts=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored',log.read_text())
  r['phases'].append({'name':name,'argv':argv,'returncode':done.returncode,'testsPassed':sum(int(a)for a,b,c in counts),'testsFailed':sum(int(b)for a,b,c in counts),'testsIgnored':sum(int(c)for a,b,c in counts),'logSha256':sha(log)});save();print(name+': return '+str(done.returncode),flush=True)
  if done.returncode:raise RuntimeError(name)
 guard();r.update(state='complete',passed=True,sourceGuardVerified=True,ownedMainMatches=True,testsPassed=sum(p['testsPassed']for p in r['phases']),testsFailed=sum(p['testsFailed']for p in r['phases']),sourceArchiveSha256=sha(Q/'audited-source.tar.gz'));save()
except BaseException as e:r.update(state='failed',error=str(e));save();raise
