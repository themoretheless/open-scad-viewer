from pathlib import Path
import hashlib,json,os,re,subprocess,tarfile
Q=Path(__file__).resolve().parent;ROOT=Q.parents[2]
STAGE=Path('/private/tmp/osv-sweep-gram-20261102-r2')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
paths=['crates/cad-predicates/src/bspline_strip_nonlinear/algebraic/overlap.rs','crates/cad-predicates/src/bspline_strip_nonlinear/algebraic/overlap/bounds.rs','crates/cad-predicates/src/bspline_strip_nonlinear/algebraic/overlap/gram.rs','crates/nurbs-core/src/join/continuity/bspline_strip/mapped_tests.rs','crates/nurbs-core/src/analysis/curve_surface_agreement.rs','crates/nurbs-core/tests/support/gram_seam_map.rs','crates/geometry-bridge/tests/gram_algebraic_body.rs']
changed={n:sha(ROOT/n)for n in paths}
ledger={str(p.relative_to(STAGE)):sha(p)for p in STAGE.rglob('*')if p.is_file()}
(Q/'source-r2.json').write_text(json.dumps(ledger,indent=2)+'\n');(Q/'changed-r2.json').write_text(json.dumps(changed,indent=2)+'\n')
r={'state':'running','passed':False,'phases':[],'sourceFileCount':len(ledger),'limitsIncreased':False,'focusInteraction':False,'wasmPublished':False,'fullGoalComplete':False,'sharedMainBuildVerified':False,'baseCommit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()}
def save():(Q/'qualification-r2.json').write_text(json.dumps(r,indent=2)+'\n')
def guard():
 assert all(sha(STAGE/n)==h for n,h in ledger.items()),'Frozen source changed'
 assert all(sha(ROOT/n)==h for n,h in changed.items()),'Owned main source changed'
env={**os.environ,'CARGO_TARGET_DIR':'/private/tmp/osv-sweep-gram-target-20261102','CARGO_BUILD_JOBS':'2','OSV_GRAM_BODY_DIR':str(Q/'native-body'),'OSV_AFFINE_FRAME_STEP_DIR':str(Q/'native-affine'),'OSV_GENERAL_CERTIFICATE_STEP':str(Q/'general-capped.step')}
phases=[('predicates',['test','-p','cad-predicates','--lib']),('continuity',['test','-p','nurbs-core','--lib','join::continuity::']),('world-traces',['test','-p','nurbs-core','--lib','curve_surface_agreement']),('brep-sweep',['test','-p','brep-core','--lib','sweep_']),('native',['test','-p','geometry-bridge','--test','authored_seam_map','--test','gram_algebraic_body','--test','bspline_boundary_joins','--test','complete_multispan_seams','--test','rational_closed_material_certificate','--test','general_boundary_certificate','--test','affine_frame_source_certificate','--test','arc_bishop_source_certificate']),('no-codec',['check','-p','cad-predicates','-p','nurbs-core','--no-default-features'])]
save()
try:
 for name,args in phases:
  guard();argv=['cargo',*args,'--manifest-path','crates/Cargo.toml','--locked','--offline']
  if args[0]=='test':argv+=['--','--test-threads=2']
  print(name+': started',flush=True)
  with (Q/(name+'.log')).open('w')as f: done=subprocess.run(argv,cwd=STAGE,env=env,stdout=f,stderr=subprocess.STDOUT)
  log=Q/(name+'.log');counts=re.findall(r'test result: ok\. (\d+) passed; 0 failed',log.read_text())
  r['phases'].append({'name':name,'argv':argv,'returncode':done.returncode,'testsPassed':sum(map(int,counts)),'logSha256':sha(log)})
  save();assert done.returncode==0 and (args[0]=='check' or counts),name
  print(name+': passed '+str(sum(map(int,counts))),flush=True)
 guard()
 with tarfile.open(Q/'qualified-source-r2.tar.gz','w:gz',compresslevel=3)as t:
  for n in ledger:t.add(STAGE/n,arcname=n,recursive=False)
 r.update(state='complete',passed=True,sourceGuardVerified=True,changedMainMatches=True,testsPassed=sum(p['testsPassed']for p in r['phases']),sourceArchiveSha256=sha(Q/'qualified-source-r2.tar.gz'));save()
except BaseException as e:r.update(state='failed',error=str(e));save();raise
