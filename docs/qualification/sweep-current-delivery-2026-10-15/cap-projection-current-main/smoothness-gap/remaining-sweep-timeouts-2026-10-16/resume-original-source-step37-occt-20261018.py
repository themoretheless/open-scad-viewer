"""Normalize absent optional QA metadata; preserve exact original STEP/models."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,datetime
OUT=Path(__file__).resolve().parent;ROOT=OUT.parents[5]
SOURCE=OUT/'v11-original-source-step37-r2-20261018';DEST=OUT/'v11-original-source-step37-occt-20261018'
RECEIPT=OUT/'v11-original-source-step37-occt-20261018.json'
PYTHON='/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'

def sha(path):
 with Path(path).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()

def main():
 assert not DEST.exists() and not RECEIPT.exists(),'Preserve all prior attempts'
 previous_path=OUT/'v11-original-source-step37-r2-20261018.json';previous=json.loads(previous_path.read_text())
 assert previous['state']=='failed' and previous['phases'][-1]['name']=='independent-opencascade'
 assert [(p['name'],p['state'])for p in previous['phases'][:2]]==[('original-source-export','passed'),('independent-rational-reference','passed')]
 for path,h in previous['inputsSha256'].items():assert sha(path)==h,path
 for phase in previous['phases']:assert sha(phase['log'])==phase['logSha256']
 assert "AttributeError: 'NoneType' object has no attribute 'get'" in Path(previous['phases'][-1]['log']).read_text()
 original_path=SOURCE/'manifest.json';original=json.loads(original_path.read_text());assert len(original['cases'])==37
 DEST.mkdir();changes=[]
 for case in original['cases']:
  step=SOURCE/case['file'];assert sha(step)==case['sha256'];shutil.copy2(step,DEST/case['file']);assert sha(DEST/case['file'])==case['sha256']
  if case.get('construction','absent')is None:
   changes.append(case['file']);case['construction']={}
 assert changes,'Require the diagnosed absent optional metadata'
 modified=DEST/'manifest.json';modified.write_text(json.dumps(original,separators=(',',':'))+'\n')
 # All geometry, source/model hashes, native reports, numeric references,
 # tolerances and expectations must remain exactly equal to the captured run.
 comparison=json.loads(modified.read_text());before=json.loads(original_path.read_text())
 for case in before['cases']:
  if case.get('construction','absent')is None:case['construction']={}
 assert comparison==before
 record={'schema':'sweep-v11-source-step37-occt-resume/1','state':'running','passed':False,'pid':os.getpid(),
  'geometryWasmSha256':previous['geometryWasmSha256'],'fullGoalComplete':False,'cases':37,
  'upstreamReceipt':str(previous_path),'upstreamReceiptSha256':sha(previous_path),
  'originalManifest':str(original_path),'originalManifestSha256':sha(original_path),'manifest':str(modified),'manifestSha256':sha(modified),
  'scriptSha256':sha(__file__),'verifierSha256':sha(ROOT/'scripts/verify-sweep-step-occt.py'),
  'metadataOnlyNormalization':{'field':'construction','before':None,'after':{},'files':changes},
  'scope':'All37unchanged original-source STEP/model/UI bindings and converged references from the passed upstream phases; only absent optional metadata normalized. The upstream whole receipt remains failed.'}
 def save():
  temp=RECEIPT.with_suffix('.writing');temp.write_text(json.dumps(record,indent=2)+'\n');temp.replace(RECEIPT)
 save()
 try:
  log=OUT/'v11-source-step37-resumed-opencascade-20261018.log'
  with log.open('w')as f:result=subprocess.run([PYTHON,'scripts/verify-sweep-step-occt.py',str(DEST)],cwd=ROOT,stdout=f,stderr=subprocess.STDOUT,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'})
  record.update(returncode=result.returncode,log=str(log),logSha256=sha(log));save()
  assert result.returncode==0,'Unchanged OCCT verifier failed; preserve all diagnostics'
  report=DEST/'opencascade-sweep.json';d=json.loads(report.read_text());assert d['passed'] and len(d['cases'])==37
  assert d['manifestSha256']==sha(modified)==record['manifestSha256']
  assert all(c['passed'] and c['native_volume_certified'] and c['native_external_material_agreement']for c in d['cases'])
  for path,h in previous['inputsSha256'].items():assert sha(path)==h,path
  assert sha(original_path)==record['originalManifestSha256']
  record.update(state='complete',passed=True,report=str(report),reportSha256=sha(report),completedAtUtc=datetime.datetime.now(datetime.timezone.utc).isoformat());save()
  print('All37original-source STEP/reference/OCCT bindings passed; only optional null metadata was normalized.',flush=True)
 except BaseException as error:
  record.update(state='failed',reason=repr(error));save();raise

if __name__=='__main__':main()
