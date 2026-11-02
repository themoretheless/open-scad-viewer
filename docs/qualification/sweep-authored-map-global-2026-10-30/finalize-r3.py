from pathlib import Path
import hashlib,json,re,shutil
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[2]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
r=json.loads((OUT/'qualification-r3.json').read_text())
assert r['state']=='complete' and r['passed'] and r['sourceGuardVerified'] and r['ownedMainMatches']
index=json.loads((OUT/'source-r2.json').read_text())
owned=json.loads((OUT/'owned-r2.json').read_text())
assert all(sha(ROOT/n)==h for n,h in owned.items())
unique=set(); binaries={}
for phase in r['phases']:
 log=OUT/('r3-'+phase['name']+'.log')
 assert sha(log)==phase['logSha256']
 body=log.read_text()
 package=None
 for line in body.splitlines():
  m=re.search(r'Running (.*?) \((.*?)\)',line)
  if m:
   label,path=m.groups();package=path.split('/')[-1]
   binaries[path]=sha(path)
  m=re.match(r'test (.*?) \.\.\. ok$',line)
  if m: unique.add((package,m.group(1)))
(OUT/'unique-tests-r3.json').write_text(json.dumps(sorted(unique),indent=2)+'\n')
r.update(uniqueTestsPassed=len(unique),binarySha256=binaries,qualificationDriverSha256=sha(OUT/'qualify-r3.py'),
 finalizerSha256=sha(Path(__file__)),readmeSha256=sha(OUT/'README.md'),openScope=['automatic discovery of arbitrary nonlinear maps','maps outside bounded positive rational B-spline representation or current work ceilings','original bodies without independently certified global reference','public WASM Rush viewport delivery and independent external STEP UI gates'])
(OUT/'result.json').write_text(json.dumps(r,indent=2)+'\n')
print(r['testsPassed'], 'executions;',r['uniqueTestsPassed'],'unique tests; guarded native qualification complete')
