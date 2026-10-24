from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import hashlib,json,subprocess
ROOT=Path('/private/tmp/osv-interpolated-physical-carriers-20261017')
OUT=Path('/private/tmp/osv-interpolated-private-matrix-20261017');OUT.mkdir(exist_ok=True)
WASM=Path('/private/tmp/osv-interpolated-carrier-private-artifact-20261017/geometry_wasm.wasm')
OLD=Path('/private/tmp/osv-general-frame-remaining-kernel-20261016')
AFFINE=Path('/private/tmp/osv-affine-physical-remaining-kernel-20261016')
GUARD=json.loads((ROOT/'docs/qualification/interpolated-carrier-source-2026-10-17/inputs.json').read_text())
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def unchanged():return all(digest(ROOT/r['path'])==r['sha256'] for r in GUARD['inputs'])
assert unchanged(); wasm_sha=digest(WASM)
rows=[]
for name,folder,harness,count in [
 ('fibered','fibered-step','fibered-source-caps',4),('caps','rational-cap-step','rational-cap-source-audit',6),('sphere','sphere-step','implicit-sphere-source',4),
 ('bishop','arc-bishop-step','arc-bishop-source',1),('general','general-frame-step','general-frame-source',1),
 ('affine','affine-frame-step','affine-physical-source',1),('guided','guided-frame-step','guided-geometric-source',1),
 ('interpolated','interpolated-carrier-step','interpolated-carrier-source',1)]:
 native=OLD if name in ['fibered','caps','sphere','bishop','general'] else AFFINE
 if name=='interpolated':native=Path('/private/tmp/osv-interpolated-carrier-remaining-kernel-20261017')
 inp=native/folder
 if count==1:inp=inp/'request.json'
 date='2026-10-17' if name=='interpolated' else '2026-10-16'
 script=ROOT/f'docs/qualification/{harness}-{date}/wasm-runtime.mts'
 rows.append((name,inp,script,count))
def run(row):
 name,inp,script,count=row;print('START',name,flush=True)
 inputs=[inp] if inp.is_file() else sorted(p for p in inp.rglob('*') if p.is_file())
 before={str(p):digest(p) for p in [script,*inputs]}
 cmd=['node','--import','tsx',str(script),str(WASM),str(inp),str(OUT/(name+'.json'))]
 log=OUT/(name+'.log')
 with log.open('w') as f:r=subprocess.run(cmd,cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
 stable=all(digest(Path(p))==h for p,h in before.items())
 report=dict(name=name,cases=count,exitCode=r.returncode,command=cmd,verifierAndProposalInputs=before,inputsUnchanged=stable,logSha256=digest(log))
 print('DONE',name,r.returncode,stable,flush=True);return report
with ThreadPoolExecutor(max_workers=2) as pool:checks=list(pool.map(run,rows))
report=dict(schema='fresh-private-native-wasm-matrix/1',sourceRevision=GUARD['revision'],inputsSha256=GUARD['inputsSha256'],sourceGuardVerified=unchanged(),wasmSha256=wasm_sha,artifactUnchanged=digest(WASM)==wasm_sha,runnerSha256=digest(Path(__file__)),checks=checks,casesPassed=sum(c['cases'] for c in checks if c['exitCode']==0),inputTreatment='Earlier Rust-generated requests are proposals only; every proof and STEP audit is recomputed by the fresh WASM. No predecessor execution credit is imported.',publicPackageQualified=False,browserQualified=False,semanticG1MatrixQualified=False,fullGoalComplete=False)
report['passed']=report['sourceGuardVerified'] and report['artifactUnchanged'] and all(c['exitCode']==0 and c['inputsUnchanged'] for c in checks)
(OUT/'summary.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k!='checks'}),flush=True)
raise SystemExit(0 if report['passed'] else 1)
