from pathlib import Path
import hashlib,json,os,re,subprocess,time
ROOT=Path(__file__).resolve().parents[3]
Q=Path(__file__).parent
OUT=Path('/private/tmp/osv-interpolated-delivery-native-workspace-20261018');OUT.mkdir(exist_ok=True)
g=json.loads((Q/'native-inputs.json').read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def guard():return all(sha(ROOT/i['path'])==i['sha256'] for i in g['inputs'])
assert guard()
env=dict(os.environ,CARGO_BUILD_JOBS='2',RUST_TEST_THREADS='2',CARGO_PROFILE_TEST_OPT_LEVEL='1',CARGO_TARGET_DIR=str(ROOT/'crates/target'))
base=['cargo','test','--locked','--manifest-path','crates/Cargo.toml','--workspace']
checks=[('catalog',['node','scripts/verify-native-test-partition.mjs',str(OUT/'catalog')]),('recognition',base+['aag','--','--test-threads=1']),('remaining',base+['--','--skip','aag'])]
r=dict(schema='interpolated-delivery-complete-native-workspace/1',sourceRevision=g['revision'],productionRustEquivalentTo=g['productionRustEquivalentTo'],inputsSha256=g['inputsSha256'],sourceGuardVerified=False,complete=False,passed=False,checks=[],qualificationClaim='none',fullGoalComplete=False)
for name,cmd in checks:
 assert guard();print('START',name,flush=True);log=OUT/(name+'.log');start=time.time()
 with log.open('w') as f:result=subprocess.run(cmd,cwd=ROOT,env=env,stdout=f,stderr=subprocess.STDOUT)
 counts=[dict(passed=int(a),failed=int(b),ignored=int(c)) for a,b,c in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log.read_text())]
 r['checks'].append(dict(name=name,command=cmd,exitCode=result.returncode,elapsedSeconds=round(time.time()-start,3),logSha256=sha(log),testResults=counts));(OUT/'summary.json').write_text(json.dumps(r,indent=2)+'\n');print('DONE',name,result.returncode,flush=True)
 if result.returncode:break
r['sourceGuardVerified']=guard();r['complete']=len(r['checks'])==len(checks);r['passed']=r['complete'] and guard() and all(c['exitCode']==0 for c in r['checks']);r['totals']={k:sum(c[k] for check in r['checks'] for c in check['testResults']) for k in ['passed','failed','ignored']}
(OUT/'summary.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps({k:v for k,v in r.items() if k!='checks'}),flush=True)
raise SystemExit(0 if r['passed'] else 1)
