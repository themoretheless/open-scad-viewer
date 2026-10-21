"""Supplemental closed API coverage after existing complete/default native gates."""
from pathlib import Path
import json,hashlib,os,subprocess,time
stage=Path('/private/tmp/osv-sweep-interval-mul-wasm-20261015');prior=Path('/private/tmp/osv-sweep-interval-mul-native-suite-20261015/receipt.json');out=Path('/private/tmp/osv-sweep-closed-annular-qualification-20261015');out.mkdir(exist_ok=True);assert not (out/'receipt.json').exists(),'Do not restart previous qualification'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();runner=stage/'scripts/check-staged-closed-annular-laws.mts';expected='6cce95b887ded1d73c8ec1886828c2fe77bfd07b34c535a38c9203299a64aabe'
r={'state':'waiting-existing-full-native-suite','passed':False,'runnerSha256':sha(runner),'driverSha256':sha(__file__),'casesRequired':16,'scope':'Additional closed annular API source/retained-model/material coverage; does not replace any207STEP/478UI/native tests'}
def save():
 p=out/'receipt.json.writing';p.write_text(json.dumps(r,indent=2)+'\n');p.replace(out/'receipt.json')
save()
while True:
 d=json.loads(prior.read_text())
 if d['state']=='complete':assert d['passed'] is True;break
 if d['state'].startswith('stopped') or d['state']=='failed':r['state']='stopped-after-prior-failure';save();raise SystemExit(1)
 try:os.kill(98811,0)
 except ProcessLookupError:r['state']='prior-ended-without-complete-pass';save();raise SystemExit(1)
 time.sleep(5)
assert sha(runner)==r['runnerSha256'];assert sha(__file__)==r['driverSha256'];assert sha(stage/'public/wasm/geometry-kernel.wasm')==expected
r.update(state='running',priorReceiptSha256=sha(prior));save();t=time.monotonic()
with (out/'closed-annular.log').open('w') as log:
 p=subprocess.run(['node','--import','tsx','--loader','./qualification-raw-imports.mjs','scripts/check-staged-closed-annular-laws.mts',str(out/'closed-annular.json'),expected],cwd=stage,env={**os.environ,'NODE_OPTIONS':'--max-old-space-size=8192'},stdout=log,stderr=subprocess.STDOUT)
r.update(state='complete' if p.returncode==0 else 'failed',passed=p.returncode==0,returncode=p.returncode,seconds=time.monotonic()-t);assert sha(runner)==r['runnerSha256'];save();raise SystemExit(p.returncode)
