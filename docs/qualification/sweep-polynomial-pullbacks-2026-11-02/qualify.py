from pathlib import Path
import hashlib,json,os,re,subprocess
Q=Path(__file__).resolve().parent
ROOT=Q.parents[2]
STAGE=Path('/private/tmp/osv-sweep-pullbacks-20261102-r1')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
paths=['crates/cad-predicates/src/bspline_strip_nonlinear/algebraic/overlap.rs','crates/cad-predicates/src/bspline_strip_nonlinear/algebraic/overlap/bounds.rs','crates/cad-predicates/src/bspline_strip_nonlinear/algebraic/overlap/pullback.rs']
changed={n:sha(ROOT/n)for n in paths}
(Q/'changed.json').write_text(json.dumps(changed,indent=2)+'\n')
ledger={str(p.relative_to(STAGE)):sha(p)for p in STAGE.rglob('*')if p.is_file() and '.git' not in p.parts and 'target' not in p.parts}
(Q/'source.json').write_text(json.dumps(ledger,indent=2)+'\n')
r={'state':'running','passed':False,'phases':[],'limitsIncreased':False,'focusInteraction':False,'wasmPublished':False,'fullGoalComplete':False,'externalStepQualified':False,'uiQualified':False,'sharedMainBuildVerified':False}
def save():(Q/'result.json').write_text(json.dumps(r,indent=2)+'\n')
def guard():
 assert all(sha(STAGE/n)==h for n,h in ledger.items())
 assert all(sha(ROOT/n)==h for n,h in changed.items())
env={**os.environ,'CARGO_TARGET_DIR':'/private/tmp/osv-sweep-circuits-20261102','CARGO_BUILD_JOBS':'2'}
phases=[('predicates',['test','-p','cad-predicates','--lib']),('continuity',['test','-p','nurbs-core','--lib','join::continuity::']),('native',['test','-p','geometry-bridge','--test','authored_seam_map']),('no-codec',['check','-p','cad-predicates','-p','nurbs-core','--no-default-features'])]
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
  print(name+': passed',flush=True)
 guard();r.update(state='complete',passed=True,sourceGuardVerified=True,changedMainMatches=True,testsPassed=sum(p['testsPassed']for p in r['phases']));save()
except BaseException as e:r.update(state='failed',error=str(e));save();raise
