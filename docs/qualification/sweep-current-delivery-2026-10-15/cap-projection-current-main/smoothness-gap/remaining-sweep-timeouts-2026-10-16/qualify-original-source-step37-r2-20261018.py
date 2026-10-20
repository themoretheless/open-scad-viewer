"""Close exact-source STEP witness gaps without changing the delivered product."""
from pathlib import Path
import datetime,hashlib,json,os,subprocess,time
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[5]
STAGE=Path('/private/tmp/osv-sweep-common-source-v11-20261018')
NODE='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
PYTHON='/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
DEST=OUT/'v11-original-source-step37-r2-20261018'
RECEIPT=OUT/'v11-original-source-step37-r2-20261018.json'
EXPECTED='68dd0ed6d368a7f4e5beb8e1a65c26720b8e6316c54a4f953e1ee49161ca4bd1'

def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def main():
 assert not RECEIPT.exists() and not DEST.exists(),'Preserve every prior run'
 assert os.getpriority(os.PRIO_PROCESS,0)==0
 ledger=json.loads((STAGE/'runtime-source-sha256.json').read_text())
 compiled={n:h for n,h in ledger.items() if n.startswith(('src/','crates/')) or n in ['package.json','package-lock.json']}
 scripts=['scripts/audit-sweep-declared-mode-coverage.mts','scripts/export-sweep-source-coverage-step.mts',
          'scripts/reference-rational-boundary-volume.py','scripts/step_rational_basis.py','scripts/verify-sweep-step-occt.py','scripts/sweep-step-provenance.mjs','scripts/verify-packed-wasm.mjs']
 inventory=OUT/'v11-declared-mode-coverage-20261018.json'
 prereqs=[OUT/'v11-primary-delivery-20261018.json',OUT/'v11-primary-runtime-verification-20261018.json',inventory,OUT/'v11-step-source-metadata-20261018.json']
 for p in prereqs[:2]:
  d=json.loads(p.read_text());assert d['state']=='complete' and d['passed'] and d['geometryWasmSha256']==EXPECTED
 inputs={str(p):sha(p)for p in [Path(__file__),*prereqs,*[ROOT/n for n in scripts],STAGE/'runtime-source-sha256.json',STAGE/'qualification-raw-imports.mjs']}
 r={'schema':'sweep-v11-original-source-step37/1','state':'waiting-runtime-load-guard','passed':False,'pid':os.getpid(),
    'geometryWasmSha256':EXPECTED,'inputsSha256':inputs,'phases':[],'fullGoalComplete':False,
    'scope':'37exact original positive UI sources, including the previously separate annular source. Native STEP exports must exactly match both existing wide/narrow UI model hashes; unchanged independent volume/OCCT controls.'}
 def save():
  temp=RECEIPT.with_suffix('.writing');temp.write_text(json.dumps(r,indent=2)+'\n');temp.replace(RECEIPT)
 def identity():
  for path,h in inputs.items():assert sha(path)==h,path
  for path,h in compiled.items():assert sha(ROOT/path)==h,path
  for path in ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm']:assert sha(ROOT/path)==EXPECTED,path
 save()
 try:
  identity();stable=None
  while stable is None or time.monotonic()-stable<30:
   stable=(stable or time.monotonic())if os.getloadavg()[0]<=8 else None
   time.sleep(2)
  commands=[('original-source-export',[NODE,'--import','tsx','--loader',str(STAGE/'qualification-raw-imports.mjs'),'scripts/export-sweep-source-coverage-step.mts',str(inventory),str(DEST)]),
            ('independent-rational-reference',[PYTHON,'scripts/reference-rational-boundary-volume.py',str(DEST)]),
            ('independent-opencascade',[PYTHON,'scripts/verify-sweep-step-occt.py',str(DEST)])]
  env={**os.environ,'PATH':str(Path(NODE).parent)+os.pathsep+os.environ['PATH'],'NODE_OPTIONS':'--max-old-space-size=8192','PYTHONDONTWRITEBYTECODE':'1'}
  for name,argv in commands:
   identity();phase={'name':name,'argv':argv,'state':'running'};r['phases'].append(phase);r['state']='running';save()
   log=OUT/f'v11-source-step37-r2-{name}-20261018.log';print(name+': started',flush=True)
   with log.open('w')as f:result=subprocess.run(argv,cwd=ROOT,env=env,stdout=f,stderr=subprocess.STDOUT)
   phase.update(state='passed'if result.returncode==0 else'failed',returncode=result.returncode,log=str(log),logSha256=sha(log));save()
   identity();assert result.returncode==0,name+' failed; preserve report/log'
   print(name+': passed',flush=True)
  manifest=DEST/'manifest.json';report=DEST/'opencascade-sweep.json';d=json.loads(report.read_text())
  assert d['passed'] is True and len(d['cases'])==37
  assert d['manifestSha256']==sha(manifest)
  assert all(c['passed'] and c['native_volume_certified'] and c['native_external_material_agreement'] for c in d['cases'])
  r.update(state='complete',passed=True,cases=37,manifest=str(manifest),manifestSha256=sha(manifest),report=str(report),reportSha256=sha(report),completedAtUtc=datetime.datetime.now(datetime.timezone.utc).isoformat());save()
  print('37original-source STEP/reference/OCCT cases passed.',flush=True)
 except BaseException as error:
  r.update(state='failed',reason=repr(error));save();raise

if __name__=='__main__':main()
