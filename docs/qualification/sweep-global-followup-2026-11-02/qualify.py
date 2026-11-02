from pathlib import Path
import os,json,hashlib,re,subprocess
Q=Path(__file__).resolve().parent;ROOT=Q.parents[2];STAGE=Path('/private/tmp/osv-sweep-followup-20261102')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
ledger=json.loads((Q/'source-final.json').read_text())
owned=['crates/brep-core/src/sweep_boundary_owned/closed.rs','crates/brep-core/src/face_contacts.rs','crates/nurbs-core/src/trim/trim_simplicity.rs','crates/nurbs-core/src/trim/trim_simplicity/exact_subdivision.rs','crates/nurbs-core/src/analysis/affine_volume.rs','crates/nurbs-core/src/sweeps/progressive_sweep/ideal_volume/atlas.rs','crates/nurbs-core/src/sweeps/progressive_sweep/ideal_volume/atlas/smooth_p_matrix.rs','crates/brep-core/src/boundary_certificate.rs','crates/brep-core/src/boundary_certificate/atlas_transport.rs','crates/geometry-bridge/src/cad_boundary_certificate.rs','crates/geometry-bridge/tests/gram_algebraic_body.rs']
def guard():
 assert all(sha(STAGE/n)==h for n,h in ledger.items()),'Frozen source changed'
 assert all(sha(ROOT/n)==ledger[n] for n in owned),'Owned main source changed'
env={**os.environ,'CARGO_TARGET_DIR':'/private/tmp/osv-sweep-gram-target-20261102','CARGO_BUILD_JOBS':'2','CARGO_PROFILE_TEST_OPT_LEVEL':'1','CARGO_PROFILE_TEST_DEBUG_ASSERTIONS':'true','CARGO_PROFILE_TEST_OVERFLOW_CHECKS':'true','OSV_GRAM_BODY_DIR':str(Q/'native-body'),'OSV_AFFINE_FRAME_STEP_DIR':str(Q/'native-affine'),'OSV_GENERAL_CERTIFICATE_STEP':str(Q/'general-capped.step')}
phases=[('topology',['test','-p','brep-topology','--lib']),('predicates',['test','-p','cad-predicates','--lib']),('continuity',['test','-p','nurbs-core','--lib','join::continuity::']),('world-traces',['test','-p','nurbs-core','--lib','curve_surface_agreement']),('trim-simplicity',['test','-p','nurbs-core','--lib','trim_simplicity']),('volume-atlas',['test','-p','nurbs-core','--lib','ideal_volume::atlas']),('affine-volume',['test','-p','nurbs-core','--lib','affine_volume']),('boundary-certificate',['test','-p','brep-core','--lib','boundary_certificate']),('brep-sweep',['test','-p','brep-core','--lib','sweep_']),('native',['test','-p','geometry-bridge','--test','authored_seam_map','--test','gram_algebraic_body','--test','bspline_boundary_joins','--test','complete_multispan_seams','--test','rational_closed_material_certificate','--test','general_boundary_certificate','--test','affine_frame_source_certificate','--test','arc_bishop_source_certificate']),('no-codec',['check','-p','cad-predicates','-p','nurbs-core','--no-default-features'])]
r={'state':'running','passed':False,'fullGoalComplete':False,'wasmPublished':False,'uiChecked':False,'phases':[]}
def save():(Q/'audit.json').write_text(json.dumps(r,indent=2)+'\n')
save()
try:
 for name,args in phases:
  guard();argv=['cargo',*args,'--manifest-path','crates/Cargo.toml','--locked','--offline']
  if args[0]=='test':argv+=['--','--test-threads=2']
  print(name+': started',flush=True);log=Q/(name+'.log')
  with log.open('w')as f:done=subprocess.run(argv,cwd=STAGE,env=env,stdout=f,stderr=subprocess.STDOUT)
  counts=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed',log.read_text())
  r['phases'].append({'name':name,'argv':argv,'returncode':done.returncode,'testsPassed':sum(int(a)for a,b in counts),'testsFailed':sum(int(b)for a,b in counts),'logSha256':sha(log)})
  save();print(name+': return '+str(done.returncode),flush=True)
 guard();r.update(state='complete',sourceGuardVerified=True,changedMainMatches=True,passed=all(p['returncode']==0 for p in r['phases']),testsPassed=sum(p['testsPassed']for p in r['phases']),testsFailed=sum(p['testsFailed']for p in r['phases']),sourceArchiveSha256=sha(Q/'audited-source.tar.gz'));save()
except BaseException as e:r.update(state='failed',error=str(e));save();raise
