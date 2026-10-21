"""Native intrinsic-interior audit after all existing complete/API gates."""
from pathlib import Path
import json,hashlib,os,subprocess,time
prior=Path('/private/tmp/osv-sweep-closed-annular-qualification-20261015/receipt.json');out=Path('/private/tmp/osv-sweep-retained-interiors-qualification-20261015');out.mkdir(exist_ok=True);assert not (out/'receipt.json').exists(),'Do not restart prior audit'
root=Path('/Users/themoretheless/Documents/Sources/open-scad-viewer');proof=root/'docs/qualification/sweep-current-delivery-2026-10-15/retained-interiors/producer.json';data=json.loads(proof.read_text());producer=Path(data['binary']);source=proof.parent/'certify_retained_surface_interiors.rs';preparer=Path('/private/tmp/osv-retained-interiors-preparer-20261015.py');full=Path('/private/tmp/osv-sweep-interval-mul-full-qualification-20261015/receipt.json');manifest=full.parent/'step207/manifest.json';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
r={'state':'waiting-existing-closed-annular-gate','passed':False,'driverSha256':sha(__file__),'preparerSha256':sha(preparer),'producerReceiptSha256':sha(proof),'scope':'Full207 captured surface-interior basis sufficient C1/C2 audit; no immersion/edge/periodic/global/source-error promotion'}
def save():
 p=out/'receipt.json.writing';p.write_text(json.dumps(r,indent=2)+'\n');p.replace(out/'receipt.json')
def identity():
 assert sha(__file__)==r['driverSha256'];assert sha(preparer)==r['preparerSha256'];assert sha(proof)==r['producerReceiptSha256'];assert sha(producer)==data['binarySha256'];assert sha(source)==data['producerSourceSha256']
identity();save()
while True:
 d=json.loads(prior.read_text())
 if d['state']=='complete':assert d['passed'] is True;break
 if d['state'].startswith('stopped') or d['state']=='failed':r['state']='stopped-after-prior-failure';save();raise SystemExit(1)
 try:os.kill(16078,0)
 except ProcessLookupError:r['state']='prior-ended-without-complete-pass';save();raise SystemExit(1)
 time.sleep(5)
identity();r.update(state='running',priorReceiptSha256=sha(prior));save();t=time.monotonic()
with (out/'audit.log').open('w') as log:
 p=subprocess.run(['python3',str(preparer),str(manifest),str(full),str(producer),str(source),str(out/'audit')],stdout=log,stderr=subprocess.STDOUT)
identity();r.update(state='complete' if p.returncode==0 else 'failed',passed=p.returncode==0,returncode=p.returncode,seconds=time.monotonic()-t);save();raise SystemExit(p.returncode)
