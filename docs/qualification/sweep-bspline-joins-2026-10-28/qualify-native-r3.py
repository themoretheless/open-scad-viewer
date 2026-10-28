from pathlib import Path
import hashlib,json,os,re,subprocess,tarfile
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[2]
STAGE=Path('/private/tmp/osv-sweep-bspline-joins-20261028-r2')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
ledger=json.loads((OUT/'source-r6-sha256.json').read_text())
assert all(sha(STAGE/name)==value for name,value in ledger.items()),'Frozen source drift before qualification'
record={'state':'running','passed':False,'stage':str(STAGE),'sourceIndexSha256':sha(OUT/'source-r6-sha256.json'),'phases':[],'limitsIncreased':False,'headed':False,'fullGoalComplete':False}
def save(): (OUT/'native-qualification-r3.json').write_text(json.dumps(record,indent=2)+'\n')
assert not (OUT/'native-qualification-r3.json').exists()
env={**os.environ,'CARGO_TARGET_DIR':'/private/tmp/osv-sweep-remaining-20261028','CARGO_BUILD_JOBS':'2'}
def run(name,args,check_only=False):
    argv=['cargo',*args,'--locked','--offline']
    if not check_only:argv+=['--','--test-threads=2']
    phase={'name':name,'argv':argv,'cwd':str(STAGE),'state':'running'};record['phases'].append(phase);save();print(name+': started',flush=True)
    logs=OUT/'verified-r3';logs.mkdir(exist_ok=True);log=logs/(name+'.log')
    with log.open('w')as f: result=subprocess.run(argv,cwd=STAGE,env=env,stdout=f,stderr=subprocess.STDOUT)
    phase.update(returncode=result.returncode,logSha256=sha(log));text=log.read_text()
    counts=[int(n)for n in re.findall(r'test result: ok\. (\d+) passed; 0 failed',text)]
    if not check_only:phase['testsPassed']=sum(counts)
    if result.returncode!=0 or not check_only and (not counts or min(counts)==0):
        phase['state']='failed';save();raise RuntimeError(name)
    phase['state']='passed';save();print(name+': passed'+(' '+str(sum(counts)) if counts else ''),flush=True)
save()
try:
    run('periodic-body',['test','--manifest-path','crates/Cargo.toml','-p','geometry-bridge','--test','bspline_boundary_joins'])
    run('predicates',['test','--manifest-path','crates/Cargo.toml','-p','cad-predicates','--lib'])
    run('topology',['test','--manifest-path','crates/Cargo.toml','-p','brep-topology','--lib'])
    run('nurbs-continuity',['test','--manifest-path','crates/Cargo.toml','-p','nurbs-core','--lib','join::continuity::'])
    run('nurbs-traces',['test','--manifest-path','crates/Cargo.toml','-p','nurbs-core','--lib','curve_surface_agreement'])
    run('brep-smoothness',['test','--manifest-path','crates/Cargo.toml','-p','brep-core','--lib','sweep_smoothness::'])
    run('closed-original-retained',['test','--manifest-path','crates/Cargo.toml','-p','brep-core','--lib','sweep_boundary_owned::closed::smooth'])
    run('embedding',['test','--manifest-path','crates/Cargo.toml','-p','brep-core','--lib','boundary_embedding::'])
    run('sphere-poles',['test','--manifest-path','crates/Cargo.toml','-p','brep-core','--lib','analytic::sphere_poles::'])
    run('cap-refusals',['test','--manifest-path','crates/Cargo.toml','-p','brep-core','--lib','sweep_miter_owned::direct_source_regressions::'])
    run('native-protocol',['test','--manifest-path','crates/Cargo.toml','-p','geometry-bridge','--test','general_boundary_certificate','--test','complete_multispan_seams'])
    run('no-codec',['check','--manifest-path','crates/Cargo.toml','-p','brep-core','--no-default-features'],True)
    assert all(sha(STAGE/name)==value for name,value in ledger.items()),'Frozen source drift after qualification'
    with tarfile.open(OUT/'qualified-source-r6.tar.gz','w:gz',compresslevel=3)as tar:
        for name in ledger:tar.add(STAGE/name,arcname=name,recursive=False)
    record.update(state='complete',passed=True,sourceGuardVerified=True,testsPassed=sum(p.get('testsPassed',0)for p in record['phases']),sourceArchiveSha256=sha(OUT/'qualified-source-r6.tar.gz'));save()
except BaseException as e:
    record.update(state='failed',error=str(e));save();raise
