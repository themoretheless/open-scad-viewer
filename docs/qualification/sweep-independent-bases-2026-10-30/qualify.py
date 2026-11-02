from pathlib import Path
import hashlib,json,os,re,subprocess,tarfile
OUT=Path(__file__).resolve().parent
STAGE=Path('/private/tmp/osv-sweep-independent-20261030-r1')
ROOT=OUT.parents[2]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
ledger=json.loads((OUT/'source-r3.json').read_text())
assert all(sha(STAGE/n)==v for n,v in ledger.items())
record={'state':'running','passed':False,'sourceIndexSha256':sha(OUT/'source-r3.json'),'phases':[],
 'limitsIncreased':False,'uiOrFocusInteraction':False,'wasmPublished':False,'fullGoalComplete':False}
def save(): (OUT/'qualification.json').write_text(json.dumps(record,indent=2)+'\n')
assert not (OUT/'qualification.json').exists()
env={**os.environ,'CARGO_TARGET_DIR':'/private/tmp/osv-sweep-remaining-20261028','CARGO_BUILD_JOBS':'2'}
def run(name,args,check=False):
 argv=['cargo',*args,'--locked','--offline']
 if not check: argv+=['--','--test-threads=2']
 phase={'name':name,'argv':argv,'cwd':str(STAGE),'state':'running'};record['phases'].append(phase);save()
 print(name+': started',flush=True)
 log=OUT/(name+'.log')
 with log.open('w')as f:r=subprocess.run(argv,cwd=STAGE,env=env,stdout=f,stderr=subprocess.STDOUT)
 counts=[int(n)for n in re.findall(r'test result: ok\. (\d+) passed; 0 failed',log.read_text())]
 phase.update(returncode=r.returncode,logSha256=sha(log),testsPassed=sum(counts))
 if r.returncode or not check and (not counts or min(counts)==0):
  phase['state']='failed';save();raise RuntimeError(name)
 phase['state']='passed';save();print(name+': passed '+str(sum(counts)),flush=True)
save()
try:
 run('predicates',['test','--manifest-path','crates/Cargo.toml','-p','cad-predicates','--lib'])
 run('continuity',['test','--manifest-path','crates/Cargo.toml','-p','nurbs-core','--lib','join::continuity::'])
 run('world-traces',['test','--manifest-path','crates/Cargo.toml','-p','nurbs-core','--lib','curve_surface_agreement'])
 run('sweep-laws-frames-bounds',['test','--manifest-path','crates/Cargo.toml','-p','nurbs-core','--lib','sweeps::'])
 run('brep-sweep-audit',['test','--manifest-path','crates/Cargo.toml','-p','brep-core','--lib','sweep_'])
 run('global-embedding',['test','--manifest-path','crates/Cargo.toml','-p','brep-core','--lib','boundary_embedding::'])
 run('global-volume',['test','--manifest-path','crates/Cargo.toml','-p','brep-core','--lib','volume_validity::'])
 run('global-nesting',['test','--manifest-path','crates/Cargo.toml','-p','brep-core','--lib','shell_nesting::'])
 run('global-intersections',['test','--manifest-path','crates/Cargo.toml','-p','brep-core','--lib','self_intersection::'])
 run('native-complete',['test','--manifest-path','crates/Cargo.toml','-p','geometry-bridge','--test','bspline_boundary_joins','--test','general_boundary_certificate','--test','complete_multispan_seams'])
 run('math-no-codec',['check','--manifest-path','crates/Cargo.toml','-p','cad-predicates','-p','nurbs-core','--no-default-features'],True)
 assert all(sha(STAGE/n)==v for n,v in ledger.items()),'Frozen source drift'
 owned=['crates/cad-predicates/src/lib.rs','crates/cad-predicates/src/bspline_strip_jets.rs','crates/cad-predicates/src/bspline_strip_spans.rs','crates/cad-predicates/src/bspline_trace_identity.rs','crates/nurbs-core/src/join/continuity/bspline_strip.rs','crates/nurbs-core/src/join/continuity/bspline_strip/tests.rs','crates/nurbs-core/src/analysis/curve_surface_agreement/bspline_trace.rs']
 assert all(sha(ROOT/n)==ledger[n] for n in owned),'Owned main source drift'
 with tarfile.open(OUT/'qualified-source-r3.tar.gz','w:gz',compresslevel=3)as tar:
  for name in ledger:tar.add(STAGE/name,arcname=name,recursive=False)
 record.update(state='complete',passed=True,sourceGuardVerified=True,ownedMainMatches=True,
  ownedSources={n:ledger[n]for n in owned},testsPassed=sum(p['testsPassed']for p in record['phases']),sourceArchiveSha256=sha(OUT/'qualified-source-r3.tar.gz'))
 save()
except BaseException as e:
 record.update(state='failed',error=str(e));save();raise
