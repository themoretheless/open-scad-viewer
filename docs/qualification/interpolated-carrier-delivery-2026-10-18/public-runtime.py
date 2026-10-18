# Transport assertions only; geometry, original proofs and STEP remain Rust.
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import hashlib,json,subprocess
ROOT=Path(__file__).resolve().parents[3];Q=Path(__file__).parent
NATIVE=ROOT/'docs/qualification/interpolated-carrier-source-2026-10-17/complete-2026-10-18/native'
OUT=Path('/private/tmp/osv-interpolated-delivery-public-runtime-20261018');OUT.mkdir(exist_ok=True)
WASM=ROOT/'public/wasm/geometry-kernel.wasm'
g=json.loads((Q/'native-inputs.json').read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def guard():return all(sha(ROOT/i['path'])==i['sha256'] for i in g['inputs'])
assert guard();wasm_sha=sha(WASM)
rows=[('fibered','fibered-step','fibered-source-caps',4),('caps','rational-cap-step','rational-cap-source-audit',6),('sphere','sphere-step','implicit-sphere-source',4),('bishop','arc-bishop-step','arc-bishop-source',1),('general','general-frame-step','general-frame-source',1),('affine','affine-frame-step','affine-physical-source',1),('guided','guided-frame-step','guided-geometric-source',1),('interpolated','interpolated-carrier-step','interpolated-carrier-source',1)]
def run(row):
 name,folder,harness,count=row;print('START',name,flush=True)
 date='2026-10-17' if name=='interpolated' else '2026-10-16'
 script=ROOT/f'docs/qualification/{harness}-{date}/wasm-runtime.mts'
 inp=NATIVE/folder
 if count==1:inp=inp/'request.json'
 paths=([inp] if inp.is_file() else sorted(p for p in inp.rglob('*') if p.is_file()))+[script]
 before={str(p):sha(p) for p in paths}
 cmd=['node','--import','tsx',str(script),str(WASM),str(inp),str(OUT/(name+'.json'))]
 log=OUT/(name+'.log')
 with log.open('w') as f:r=subprocess.run(cmd,cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
 report=dict(name=name,cases=count,exitCode=r.returncode,command=cmd,inputs=before,inputsUnchanged=all(sha(Path(p))==h for p,h in before.items()),logSha256=sha(log))
 print('DONE',name,r.returncode,flush=True);return report
with ThreadPoolExecutor(max_workers=2) as pool:checks=list(pool.map(run,rows))
report=dict(schema='interpolated-source-public-wasm-runtime-matrix/1',sourceRevision=g['revision'],productionRustEquivalentTo=g['productionRustEquivalentTo'],nativeInputsSha256=g['inputsSha256'],sourceGuardVerified=guard(),wasmSha256=wasm_sha,wasmBytes=WASM.stat().st_size,artifactUnchanged=sha(WASM)==wasm_sha,checks=checks,casesPassed=sum(c['cases'] for c in checks if c['exitCode']==0),runnerSha256=sha(Path(__file__)),browserQualified=False,semanticG1MatrixQualified=False,fullGoalComplete=False)
report['passed']=report['sourceGuardVerified'] and report['artifactUnchanged'] and all(c['exitCode']==0 and c['inputsUnchanged'] for c in checks)
(OUT/'summary.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k!='checks'}),flush=True)
raise SystemExit(0 if report['passed'] else 1)
