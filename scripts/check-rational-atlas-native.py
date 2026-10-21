#!/usr/bin/env python3
"""Replay WASM-produced source and enclosure certificates in standalone Rust."""
import sys,json,subprocess,hashlib,time
from pathlib import Path
checker,inputs,output=sys.argv[1:]
r=json.loads(Path(inputs).read_text())
report={'wasmSha256':r['wasmSha256'],'checkerSha256':hashlib.sha256(Path(checker).read_bytes()).hexdigest(),'proofs':[],'forgeries':[]}
def save():Path(output).write_text(json.dumps(report,indent=2)+'\n')
def run(mode,request):
 t=time.monotonic()
 p=subprocess.run([checker,mode],input=json.dumps(request,allow_nan=False),text=True,capture_output=True,timeout=600)
 return p,time.monotonic()-t
save()
if 'sourceProofs' not in r:
 for f in r['forgeries']:
  p,seconds=run(f['mode'],f['request'])
  assert p.returncode!=0,(f['name'],f['mutation'],'forgery accepted')
  report['forgeries'].append({'name':f['name'],'mutation':f['mutation'],'seconds':seconds,'rejection':p.stderr.strip()});save()
 print(json.dumps({'forgeriesRejected':len(report['forgeries']),'wasmSha256':r['wasmSha256']}));sys.exit(0)
for case in r['sourceProofs']+r['sumProofs']:
 p,seconds=run(case['mode'],case['request'])
 assert p.returncode==0,(case['name'],p.stderr)
 v=json.loads(p.stdout);assert v['status']=='verified'
 assert v['certificate']==case['request']['certificate']
 if 'sourceApproximationTopology' in case:assert v['sourceApproximationTopology']==case['sourceApproximationTopology']
 report['proofs'].append({'name':case['name'],'seconds':seconds,'status':v['status']});save()
 for f in case.get('forgeries',[]):
  p,seconds=run(case['mode'],f['request'])
  assert p.returncode!=0,(case['name'],f['mutation'],'forgery accepted')
  report['forgeries'].append({'name':case['name'],'mutation':f['mutation'],'seconds':seconds,'rejection':p.stderr.strip()});save()
print(json.dumps({'proofs':len(report['proofs']),'forgeriesRejected':len(report['forgeries']),'wasmSha256':r['wasmSha256']},indent=2))
