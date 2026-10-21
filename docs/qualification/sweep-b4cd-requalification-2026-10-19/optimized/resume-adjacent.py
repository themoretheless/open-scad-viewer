"""Transport every original planar proof request, including curve modes."""
from pathlib import Path
import hashlib,json,os,subprocess
OUT=Path(__file__).resolve().parent;STAGE=Path('/private/tmp/osv-sweep-b4cd-interval-candidate-20261019')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
old=OUT/'adjacent-preservation.json';prior=json.loads(old.read_text())
assert prior['state']=='failed'and prior['error']=="'a'"and all(p['state']=='passed'for p in prior['phases'])
checker=OUT/'independent-original-checker/verify_profile_certificate';assert sha(checker)==prior['checkerSha256']
source=OUT/'planar-preservation.json';raw=json.loads(source.read_text());assert raw['wasmSha256']==prior['geometryWasmSha256']and len(raw['proofs'])==15
normalized={'wasmSha256':raw['wasmSha256'],'proofs':[],'refusals':[]}
for p in raw['proofs']:
 request={k:v for k,v in p.items()if k not in ['name','mode','verified','forged']}
 assert request['certificate']==p['certificate']
 forged={**request,'certificate':p['forged']}
 normalized['proofs'].append({'name':p['name'],'mode':p['mode'],'request':request,'verified':p['verified'],
  'forgeries':[{'mutation':'retained-original-forgery','request':forged}]})
norm=OUT/'planar-native-complete-input.json';assert not norm.exists();norm.write_text(json.dumps(normalized)+'\n')
record={'state':'running','passed':False,'geometryWasmSha256':prior['geometryWasmSha256'],'checkerSha256':sha(checker),
 'stoppedReceiptSha256':sha(old),'originalPlanarCorpusSha256':sha(source),'transportInputSha256':sha(norm),
 'scope':'Original planar requests preserved in full for each existing native checker mode; no geometry, certificate, tolerance or admission changes.'}
receipt=OUT/'adjacent-preservation-resumed.json';assert not receipt.exists();receipt.write_text(json.dumps(record,indent=2)+'\n')
try:
 with (OUT/'planar-independent-resumed.log').open('w')as log:r=subprocess.run(['python3','scripts/check-certified-followup.py',str(norm),str(OUT/'planar-independent.json'),'--checker',str(checker)],cwd=STAGE,stdout=log,stderr=subprocess.STDOUT,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'})
 assert r.returncode==0
 reports=[json.loads((OUT/name).read_text())for name in ['branches-independent.json','polyhedral-independent.json','planar-independent.json']]
 rows=[p for report in reports for p in (report['proofs']if isinstance(report,dict)else report)]
 assert len(rows)==36 and sum(p['forgedRejected']for p in rows)==92
 assert sha(old)==record['stoppedReceiptSha256']and sha(source)==record['originalPlanarCorpusSha256']and sha(checker)==record['checkerSha256']
 record.update(state='complete',passed=True,certificates=36,forgedRejected=92,expectedRefusals=3,
  nativeReportSha256={name:sha(OUT/name)for name in ['branches-independent.json','polyhedral-independent.json','planar-independent.json']})
except BaseException as e:record.update(state='failed',passed=False,error=str(e));raise
finally:receipt.write_text(json.dumps(record,indent=2)+'\n')
