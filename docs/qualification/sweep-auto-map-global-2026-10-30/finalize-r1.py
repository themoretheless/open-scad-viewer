from pathlib import Path
import hashlib,json,re,shutil
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[2]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
r=json.loads((OUT/'qualification-r1.json').read_text())
assert r['state']=='complete' and r['passed'] and r['sourceGuardVerified'] and r['ownedMainMatches']
assert sha(OUT/'qualify-native-rational.py')==r['extraDriverSha256']
index=json.loads((OUT/'source-r1.json').read_text())
owned=json.loads((OUT/'owned-r1.json').read_text())
assert all(sha(ROOT/n)==h for n,h in owned.items())
unique=set(); binaries={}
for phase in r['phases']:
 log=OUT/('r1-'+phase['name']+'.log')
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
(OUT/'unique-tests-r1.json').write_text(json.dumps(sorted(unique),indent=2)+'\n')
r.update(sharedMainBuildVerified=False,nativeProtocolTestsPassed=sum(p["testsPassed"]for p in r["phases"]if p["name"].startswith("native-")),uniqueTestsPassed=len(unique),binarySha256=binaries,qualificationDriverSha256=sha(OUT/'qualify-r1.py'),
 finalizerSha256=sha(Path(__file__)),readmeSha256=sha(OUT/'README.md'),openScope=['rational inverse charts beyond the searched linear and quadratic homogeneous features','maps outside the existing exact-work ceiling or positive ordered covering predicates','original bodies outside the available direct or independent-reference global atlases','trimmed material whose carrier rectangle folds or crosses the axis','public WASM Rush viewport delivery and independent external STEP UI gates'])
(OUT/'result.json').write_text(json.dumps(r,indent=2)+'\n')
print(r['testsPassed'], 'executions;',r['uniqueTestsPassed'],'unique tests; guarded native qualification complete')
