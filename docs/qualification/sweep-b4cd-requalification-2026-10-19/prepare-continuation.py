"""Supply a fresh build-only identity adapter and preserve the stopped QA attempt."""
from pathlib import Path
import hashlib,json
OUT=Path(__file__).resolve().parent;DEST=OUT/'optimized'
STAGE=Path('/private/tmp/osv-sweep-b4cd-interval-candidate-20261019')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
prior_path=DEST/'receipt.json';prior=json.loads(prior_path.read_text())
assert prior['state']=='failed'and prior['passed']is False
assert [(p['name'],p['state'])for p in prior['phases']]==[
 ('original-two-files','passed'),('public100','passed'),('native-producers','passed'),('multispan-constructed','failed')]
assert prior['public100']['passedTests']==prior['public100']['tests']==736
failure=(DEST/'multispan-constructed.log').read_text()
assert 'ENOENT'in failure and'completed-build-receipt.json'in failure
adapter=STAGE/'completed-build-receipt.json';assert not adapter.exists()
raw=STAGE/'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'
packed=STAGE/'public/wasm/geometry-kernel.wasm'
assert sha(packed)==prior['geometryWasmSha256']
log=OUT/'interval-wasm-build.log';text=log.read_text()
assert 'Finished `release` profile'in text and 'wasm-opt: 13522418 -> 12070879 bytes'in text
assert raw.stat().st_size==13522418 and packed.stat().st_size==12070879
adapter.write_text(json.dumps({'schema':'completed-isolated-geometry-build/1','passed':True,
 'scope':'Completed compiler and packaging identity only; no geometric qualification or full-goal claim.',
 'qualified':False,'fullGoalComplete':False,'buildLogSha256':sha(log),
 'sourceLedgerSha256':sha(STAGE/'qualification-inputs.json'),
 'artifacts':{name:{'sha256':sha(p),'byteLength':p.stat().st_size}for name,p in [('raw',raw),('packed',packed)]}},indent=2)+'\n')
source=DEST/'run.py';code=source.read_text()
assert sha(source)==prior['driverSha256']
def replace(before,after):
 global code
 assert code.count(before)==1,before
 code=code.replace(before,after)
replace("receipt_path = OUT / 'receipt.json'","receipt_path = OUT / 'continuation.json'")
replace("'schema': 'sweep-b4cd-requalification/1'","'schema': 'sweep-b4cd-requalification-continuation/1'")
start=code.index("    tests('original-two-files'")
end=code.index("    producer = STAGE /",start)
code=code[:start]+"    assert prior['public100']['passedTests'] == prior['public100']['tests'] == 736\n"+code[end:]
replace("diagnostic = OUT / 'multispan'","diagnostic = OUT / 'multispan-resumed'")
replace("try:\n    save()",'''prior_path = OUT / 'receipt.json'
prior = json.loads(prior_path.read_text())
assert prior['state'] == 'failed' and prior['phases'][-1]['name'] == 'multispan-constructed'
assert all(p['state'] == 'passed' for p in prior['phases'][:-1])
assert 'completed-build-receipt.json' in (OUT / 'multispan-constructed.log').read_text()
record['completedPrerequisites'] = {'receipt': str(prior_path), 'sha256': sha(prior_path),
    'publicReportSha256': sha(OUT / 'public100.json'), 'public100': prior['public100'],
    'buildAdapterSha256': sha(STAGE / 'completed-build-receipt.json')}
try:
    save()''')
replace("    assert sha(SNAPSHOT) == record['snapshotSha256']",'''    assert sha(SNAPSHOT) == record['snapshotSha256']
    if 'completedPrerequisites' in record:
        previous = record['completedPrerequisites']
        assert sha(previous['receipt']) == previous['sha256']
        assert sha(OUT / 'public100.json') == previous['publicReportSha256']
        assert sha(STAGE / 'completed-build-receipt.json') == previous['buildAdapterSha256']''')
resume=DEST/'resume.py';assert not resume.exists();resume.write_text(code)
supplement=(DEST/'supplements.py').read_text()
assert supplement.count("OUT / 'receipt.json'")==2
(DEST/'supplements-continuation.py').write_text(supplement.replace("OUT / 'receipt.json'","OUT / 'continuation.json'"))
(DEST/'continuation-preparation.json').write_text(json.dumps({'passed':True,'scope':'Metadata-only continuation after an authenticated pre-geometry ENOENT; stopped parent remains failed.',
 'priorReceiptSha256':sha(prior_path),'priorDriverSha256':sha(source),'newDriverSha256':sha(resume),
 'buildAdapterSha256':sha(adapter),'preparerSha256':sha(__file__),'geometryAndBudgetsChanged':False},indent=2)+'\n')
print('Fresh compiler artifact identity supplied; preserved all prior successes and the stopped parent.')
