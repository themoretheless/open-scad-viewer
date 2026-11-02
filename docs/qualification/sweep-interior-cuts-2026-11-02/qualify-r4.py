from pathlib import Path
import hashlib,json,os,re,subprocess,tarfile
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[2]
STAGE=Path('/private/tmp/osv-sweep-interior-cuts-20261102-r4')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
ledger=json.loads((OUT/'source-r4.json').read_text())
changed=json.loads((OUT/'changed-paths.json').read_text())
record={'passed':False,'state':'running','sourceIndexSha256':sha(OUT/'source-r4.json'),'phases':[], 'limitsIncreased':False,'uiOrFocusInteraction':False,'wasmPublished':False,'fullGoalComplete':False,'sharedMainBuildVerified':False}
def save(): (OUT/'result.json').write_text(json.dumps(record,indent=2)+'\n')
def guard():
 assert all(sha(STAGE/n)==v for n,v in ledger.items()), 'Frozen source drift'
 assert all(sha(ROOT/n)==v for n,v in changed.items()), 'Changed main source drift'
env={**os.environ,'CARGO_TARGET_DIR':'/private/tmp/osv-sweep-cuts-20261102','CARGO_BUILD_JOBS':'2'}
def run(name,args,check=False):
 guard()
 argv=['cargo',*args,'--locked','--offline']
 if not check:argv+=['--','--test-threads=2']
 phase={'name':name,'argv':argv,'state':'running'};record['phases'].append(phase);save()
 print(name+': started',flush=True)
 log=OUT/(name+'.log')
 with log.open('w') as f:r=subprocess.run(argv,cwd=STAGE,env=env,stdout=f,stderr=subprocess.STDOUT)
 counts=[int(n)for n in re.findall(r'test result: ok\. (\d+) passed; 0 failed',log.read_text())]
 phase.update(returncode=r.returncode,logSha256=sha(log),testsPassed=sum(counts))
 if r.returncode or not check and (not counts or min(counts)==0):
  phase['state']='failed';save();raise RuntimeError(name)
 phase['state']='passed';save();print(name+': passed '+str(sum(counts)),flush=True)
assert not (OUT/'result.json').exists()
save()
try:
 run('predicates',['test','--manifest-path','crates/Cargo.toml','-p','cad-predicates','--lib'])
 run('continuity',['test','--manifest-path','crates/Cargo.toml','-p','nurbs-core','--lib','join::continuity::'])
 run('native-seams',['test','--manifest-path','crates/Cargo.toml','-p','geometry-bridge','--test','authored_seam_map','--test','bspline_boundary_joins','--test','complete_multispan_seams'])
 run('no-codec',['check','--manifest-path','crates/Cargo.toml','-p','cad-predicates','-p','nurbs-core','--no-default-features'],True)
 guard()
 with tarfile.open(OUT/'qualified-source-r4.tar.gz','w:gz',compresslevel=3)as tar:
  for n in ledger:tar.add(STAGE/n,arcname=n,recursive=False)
 record.update(state='complete',passed=True,sourceGuardVerified=True,changedMainMatches=True,testsPassed=sum(p['testsPassed']for p in record['phases']),sourceArchiveSha256=sha(OUT/'qualified-source-r4.tar.gz'))
 save()
except BaseException as e:
 record.update(state='failed',error=str(e));save();raise
