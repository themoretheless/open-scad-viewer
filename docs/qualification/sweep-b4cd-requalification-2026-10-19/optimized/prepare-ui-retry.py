"""Preserve the resolution failure; restart the unchanged UI on an actual isolated package."""
from pathlib import Path
import hashlib,json
OUT=Path(__file__).resolve().parent;sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
prior_path=OUT/'finish.json';prior=json.loads(prior_path.read_text());assert prior['state']=='failed'
assert [(p['name'],p['state'])for p in prior['phases']]==[('c2-step4-export','passed'),('c2-step4-reference','passed'),('c2-step4-occt','passed'),('full-ui478','failed')]
assert 'E_QUALIFICATION_PLAYWRIGHT_RESOLUTION_ESCAPE'in(OUT/'ui478-complete/closed-frenet.log').read_text()
assert not(OUT/'ui478-complete/closed-frenet/matrix.json').exists()
loaded=json.loads((OUT/'browser-package-loaded.json').read_text());assert loaded['passed']and loaded['headless']
source=OUT/'finish.py';assert sha(source)==prior['driverSha256'];code=source.read_text()
def replace(before,after):
 global code
 assert code.count(before)==1,before;code=code.replace(before,after)
replace("path=OUT/'finish.json'","path=OUT/'finish-retry2.json'")
start=code.index(" run('c2-step4-export'");end=code.index(" ui=OUT/",start)
code=code[:start]+''' old_finish=OUT/'finish.json'
 old_finish_data=json.loads(old_finish.read_text())
 assert old_finish_data['state']=='failed'and all(p['state']=='passed'for p in old_finish_data['phases'][:-1])
 c2_report=json.loads((c2/'opencascade-sweep.json').read_text())
 assert c2_report['passed']and len(c2_report['cases'])==4
 record['completedC2']={'receipt':str(old_finish),'receiptSha256':sha(old_finish),'manifestSha256':sha(c2/'manifest.json'),'occtSha256':sha(c2/'opencascade-sweep.json')}
 record['actualBrowserPackageLoadSha256']=sha(OUT/'browser-package-loaded.json');save()
'''+code[end:]
replace("ui=OUT/'ui478-complete'","ui=OUT/'ui478-complete-retry2'")
replace(" assert sha(__file__)==record['driverSha256']",''' assert sha(__file__)==record['driverSha256']
 if 'actualBrowserPackageLoadSha256'in record:assert sha(OUT/'browser-package-loaded.json')==record['actualBrowserPackageLoadSha256']
 if 'completedC2'in record:
  c=record['completedC2'];assert sha(c['receipt'])==c['receiptSha256']and sha(OUT/'c2-step4/manifest.json')==c['manifestSha256']and sha(OUT/'c2-step4/opencascade-sweep.json')==c['occtSha256']''')
target=OUT/'finish-retry2.py';assert not target.exists();target.write_text(code)
supp=(OUT/'supplements.py').read_text();assert supp.count("OUT / 'receipt.json'")==2
supp=supp.replace("OUT / 'receipt.json'","OUT / 'finish-retry2.json'")
before="canonical = OUT / 'canonical207-ui478'";assert supp.count(before)==1
supp=supp.replace(before,"canonical = OUT / 'canonical-complete'")
(OUT/'supplements-final.py').write_text(supp)
(OUT/'ui-retry-preparation.json').write_text(json.dumps({'passed':True,'previousDriverSha256':sha(source),'previousReceiptSha256':sha(prior_path),'newDriverSha256':sha(target),'actualBrowserPackageLoadSha256':sha(OUT/'browser-package-loaded.json'),'scope':'Environment-only retry with complete preserved geometry gates and unchanged headless native-running criteria.'},indent=2)+'\n')
