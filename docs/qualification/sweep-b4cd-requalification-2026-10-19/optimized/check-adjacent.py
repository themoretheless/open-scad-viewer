"""Preserve the installed Minkowski/branch functionality across the Rust fix."""
from pathlib import Path
import hashlib,json,os,subprocess,time
OUT=Path(__file__).resolve().parent
STAGE=Path('/private/tmp/osv-sweep-b4cd-interval-candidate-20261019')
NODE='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
CHECKER=OUT/'independent-original-checker/verify_profile_certificate'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
assert not (OUT/'adjacent-preservation.json').exists()
kernel=sha(STAGE/'public/wasm/geometry-kernel.wasm')
record={'state':'running','passed':False,'geometryWasmSha256':kernel,'checkerSha256':sha(CHECKER),'driverSha256':sha(__file__),'phases':[],'scope':'Fresh current-WASM certificates independently checked by the preserved original b4cd Rust verifier; no universal topology or sweep qualification claim.'}
def save():(OUT/'adjacent-preservation.json').write_text(json.dumps(record,indent=2)+'\n')
def run(name,argv):
 assert sha(CHECKER)==record['checkerSha256']and sha(STAGE/'public/wasm/geometry-kernel.wasm')==kernel
 p={'name':name,'argv':list(map(str,argv)),'state':'running'};record['phases'].append(p);save();print(name+': started',flush=True);start=time.monotonic()
 with (OUT/(name+'.log')).open('w')as log:r=subprocess.run(p['argv'],cwd=STAGE,stdout=log,stderr=subprocess.STDOUT,env={**os.environ,'NODE_OPTIONS':'--max-old-space-size=8192','PYTHONDONTWRITEBYTECODE':'1'})
 p.update(state='passed'if r.returncode==0 else'failed',seconds=time.monotonic()-start,returncode=r.returncode);save();assert r.returncode==0,name
try:
 save()
 corpus=OUT/'minkowski-branches-preservation.json';d=json.loads(corpus.read_text());assert d['wasmSha256']==kernel and len(d['proofs'])==13 and sum(len(p['forgeries'])for p in d['proofs'])==45 and len(d['refusals'])==3
 record['branchesCorpusSha256']=sha(corpus)
 run('branches-independent',['python3','scripts/check-certified-followup.py',corpus,OUT/'branches-independent.json','--checker',CHECKER])
 poly=OUT/'polyhedral-preservation.json'
 run('polyhedral-wasm',[NODE,'--import','tsx','scripts/check-minkowski3d.mts','public/wasm/geometry-kernel.wasm',poly])
 run('polyhedral-independent',['python3','scripts/check-minkowski3d-certificates.py',poly,OUT/'polyhedral-independent.json','--checker',CHECKER])
 planar=OUT/'planar-preservation.json'
 run('planar-wasm',[NODE,'--import','tsx','scripts/check-minkowski-regions.mts','public/wasm/geometry-kernel.wasm',planar])
 raw=json.loads(planar.read_text());assert raw['wasmSha256']==kernel and len(raw['proofs'])==15
 normalized={'wasmSha256':kernel,'proofs':[{'name':p['name'],'mode':p['mode'],'request':{k:p[k]for k in ['a','b','certificate']},'verified':p['verified'],'forgeries':[{'mutation':'retained-original-forgery','request':{'a':p['a'],'b':p['b'],'certificate':p['forged']}}]}for p in raw['proofs']],'refusals':[]}
 norm=OUT/'planar-native-input.json';norm.write_text(json.dumps(normalized)+'\n')
 run('planar-independent',['python3','scripts/check-certified-followup.py',norm,OUT/'planar-independent.json','--checker',CHECKER])
 reports=[json.loads((OUT/name).read_text())for name in ['branches-independent.json','polyhedral-independent.json','planar-independent.json']]
 rows=[p for report in reports for p in (report['proofs']if isinstance(report,dict)else report)]
 assert len(rows)==36 and sum(p['forgedRejected']for p in rows)==92
 record.update(state='complete',passed=True,certificates=36,forgedRejected=92,expectedRefusals=3,
  sourceReportSha256={name:sha(OUT/name)for name in ['minkowski-branches-preservation.json','polyhedral-preservation.json','planar-preservation.json']})
 save()
except BaseException as error:record.update(state='failed',passed=False,error=str(error));save();raise
