from pathlib import Path
import hashlib,json,os,statistics,subprocess,time
OUT=Path(__file__).resolve().parent
BASE=Path('/private/tmp/osv-sweep-b4cd-requalification-20261019')
CANDIDATE=Path('/private/tmp/osv-sweep-b4cd-interval-candidate-20261019')
NODE='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
SOURCE='periodic-fixed-normal-arc-affine-hollow-station-g2.r'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
dest=OUT/'paired-interval';dest.mkdir(exist_ok=False)
inputs={str(p):sha(p)for p in [Path(__file__),OUT/'measure-candidate.mts',*[root/name for root in [BASE,CANDIDATE] for name in ['public/wasm/geometry-kernel.wasm','examples/rush/'+SOURCE,'crates/nurbs-core/src/numerics/distance_bounds.rs']]]}
record={'state':'running','passed':False,'inputsSha256':inputs,'pairs':[],'source':SOURCE,'limitsChanged':False,'scope':'Four alternating fresh-process pairs; actual model, snapshot, construction and admission receipts compared by SHA256.'}
def save():(dest/'receipt.json').write_text(json.dumps(record,indent=2)+'\n')
try:
 save()
 for index,order in enumerate([['baseline','candidate'],['candidate','baseline'],['baseline','candidate'],['candidate','baseline']]):
  pair={'order':order,'runs':{}};record['pairs'].append(pair);save()
  for label in order:
   root=BASE if label=='baseline'else CANDIDATE;path=dest/f'{index}-{label}.json'
   for name,h in inputs.items():assert sha(name)==h,name
   load=os.getloadavg();start=time.monotonic()
   with (dest/f'{index}-{label}.log').open('w')as log:
    result=subprocess.run([NODE,'--import','tsx','--loader',str(root/'qualification-raw-imports.mjs'),str(OUT/'measure-candidate.mts'),str(root),SOURCE,str(path)],cwd=root,stdout=log,stderr=subprocess.STDOUT,env={**os.environ,'NODE_OPTIONS':'--max-old-space-size=8192'})
   assert result.returncode==0,label
   d=json.loads(path.read_text());assert d['passed']
   pair['runs'][label]={'report':str(path),'sha256':sha(path),'buildMs':d['buildMs'],'wallSeconds':time.monotonic()-start,'loadBefore':load,'loadAfter':os.getloadavg()};save()
   print(index,label,round(d['buildMs'],2),flush=True)
  before,after=[json.loads(Path(pair['runs'][label]['report']).read_text())for label in ['baseline','candidate']]
  for key in ['sourceSha256','modelSha256','snapshotSha256','constructionSha256','admissionSha256']:assert before[key]==after[key],key
  pair['improvementPercent']=(1-after['buildMs']/before['buildMs'])*100;pair['identicalNativeReceipts']=True;save()
 record['medianImprovementPercent']=statistics.median(p['improvementPercent']for p in record['pairs'])
 record['allPairsFaster']=all(p['improvementPercent']>0 for p in record['pairs'])
 record['passed']=record['allPairsFaster']and record['medianImprovementPercent']>3
 record['state']='complete'if record['passed']else'rejected-no-consistent-win';save()
except BaseException as error:
 record.update(state='failed',passed=False,error=str(error));save();raise
