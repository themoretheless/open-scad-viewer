from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import hashlib,json,subprocess
ROOT=Path('/private/tmp/osv-interpolated-physical-carriers-20261017')
NATIVE=Path('/private/tmp/osv-interpolated-carrier-remaining-kernel-20261017')
OUT=Path('/private/tmp/osv-interpolated-full-independent-matrix-20261018');OUT.mkdir(exist_ok=True)
PYTHON='/private/tmp/osv-rational-occt-20261016/bin/python'
WASM=Path('/private/tmp/osv-interpolated-carrier-private-artifact-20261017/geometry_wasm.wasm')
guard=json.loads((ROOT/'docs/qualification/interpolated-carrier-source-2026-10-17/inputs.json').read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def unchanged():return all(sha(ROOT/r['path'])==r['sha256'] for r in guard['inputs'])
assert unchanged();native=json.loads((NATIVE/'summary.json').read_text());assert native['passed'] and native['sourceGuardVerified'];assert native['frozenCodeRevision']==guard['revision']
prior=json.loads(Path('/private/tmp/osv-interpolated-private-matrix-20261017/summary.json').read_text());assert prior['passed'] and prior['casesPassed']==19 and prior['wasmSha256']==sha(WASM)
rows=[('fibered','fibered-step','scripts/verify-fibered-source-step.py',4),('caps','rational-cap-step','scripts/verify-rational-filled-cap-step.py',6),('sphere','sphere-step','scripts/verify-implicit-sphere-step.py',4),('bishop','arc-bishop-step','docs/qualification/arc-bishop-source-2026-10-16/occt.py',1),('general','general-frame-step','docs/qualification/arc-bishop-source-2026-10-16/occt.py',1),('affine','affine-frame-step','docs/qualification/arc-bishop-source-2026-10-16/occt.py',1),('guided','guided-frame-step','docs/qualification/arc-bishop-source-2026-10-16/occt.py',1),('interpolated','interpolated-carrier-step','scripts/verify-interpolated-carrier-step.py',1)]
correspondence=[]
for name,folder,verifier,count in rows:
 previous=next(c for c in prior['checks'] if c['name']==name)
 if name=='fibered':inputs=sorted((NATIVE/folder).glob('*/source.json'))
 elif name in ['caps','sphere']:inputs=sorted((NATIVE/folder).glob('*.source.json'))
 else:inputs=[NATIVE/folder/'request.json']
 assert len(inputs)==count,(name,len(inputs),count)
 script=Path(previous['command'][3]);assert sha(script)==previous['verifierAndProposalInputs'][str(script)]
 for p in inputs:
  rel=p.relative_to(NATIVE/folder)
  earlier=[(q,h) for q,h in previous['verifierAndProposalInputs'].items() if q.endswith('/'+str(rel))]
  assert len(earlier)==1,(name,rel,earlier)
  assert sha(p)==earlier[0][1],f'Fresh Rust proposal differs: {p}'
  correspondence.append({'case':name,'freshNativeInput':str(p),'sha256':sha(p),'identicalToExecutedPrivateWasmProposal':earlier[0][0]})

def run(row):
 name,folder,verifier,count=row;print('START',name,flush=True)
 script=ROOT/verifier;before=sha(script)
 inputs={str(p):sha(p) for p in sorted((NATIVE/folder).rglob('*')) if p.is_file()}
 cmd=[PYTHON,str(script),str(NATIVE/folder),str(OUT/(name+'.json'))];log=OUT/(name+'.log')
 with log.open('w') as f:r=subprocess.run(cmd,cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
 report=dict(name=name,cases=count,command=cmd,exitCode=r.returncode,verifierSha256=before,inputs=inputs,inputsUnchanged=sha(script)==before and all(sha(Path(p))==h for p,h in inputs.items()),logSha256=sha(log))
 print('DONE',name,r.returncode,flush=True);return report
with ThreadPoolExecutor(max_workers=2) as pool:checks=list(pool.map(run,rows))
report=dict(schema='interpolated-source-fresh-independent-step-matrix/1',sourceRevision=guard['revision'],inputsSha256=guard['inputsSha256'],sourceGuardVerified=unchanged(),nativeQualifierPassed=True,nativeTestsPassed=native['totals']['passed'],wasmSha256=sha(WASM),privateWasmCasesPassed=19,freshNativeProposalCorrespondence=correspondence,checks=checks,independentOcctCasesPassed=sum(c['cases'] for c in checks if c['exitCode']==0),runnerSha256=sha(Path(__file__)),publicPackageQualified=False,browserQualified=False,semanticG1MatrixQualified=False,fullGoalComplete=False)
report['passed']=report['sourceGuardVerified'] and all(c['exitCode']==0 and c['inputsUnchanged'] for c in checks)
(OUT/'summary.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k not in ['checks','freshNativeProposalCorrespondence']}),flush=True)
raise SystemExit(0 if report['passed'] else 1)
