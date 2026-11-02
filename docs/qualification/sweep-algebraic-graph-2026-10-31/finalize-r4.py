from pathlib import Path
import hashlib,json,re,shutil
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[2]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
r=json.loads((OUT/'qualification-r4.json').read_text())
assert r['state']=='complete' and r['passed'] and r['sourceGuardVerified'] and r['ownedMainMatches']

index=json.loads((OUT/'source-r4.json').read_text())
owned=json.loads((OUT/'owned-r4.json').read_text())
assert all(sha(ROOT/n)==h for n,h in owned.items())
unique=set(); binaries={}
for phase in r['phases']:
 log=OUT/('r4-'+phase['name']+'.log')
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
(OUT/'unique-tests-r4.json').write_text(json.dumps(sorted(unique),indent=2)+'\n')
r.update(developmentEvidenceIndexSha256=sha(OUT/'development-evidence.json'),sharedMainBuildVerified=False,nativeProtocolTestsPassed=sum(p["testsPassed"]for p in r["phases"]if p["name"].startswith("native-")),uniqueTestsPassed=len(unique),binarySha256=binaries,qualificationDriverSha256=sha(OUT/'qualify-r4.py'),
 finalizerSha256=sha(Path(__file__)),freezeDriverSha256=sha(OUT/'freeze-r4.py'),readmeSha256=sha(OUT/'README.md'),changedSourcesSha256=sha(OUT/'changed-sources.json'),incrementalPatchSha256=sha(OUT/'change-from-previous-qualified.patch'),openScope=['algebraic maps without a common globally monotone Cartesian coordinate or a covered existing exact chart',
 'algebraic residuals outside exact bivariate divisibility and the shared work ceiling',
 'nonplanar carriers outside the exact single-span polynomial graph class and available independent atlases',
 'moving-matrix and closed-time nonplanar graph cases beyond the existing atlases',
 'new graph theorem does not independently bind material BRep caps or grant joint G1 G2',
 'public WASM Rush viewport delivery and independent external STEP UI gates'])
(OUT/'result.json').write_text(json.dumps(r,indent=2)+'\n')
print(r['testsPassed'], 'executions;',r['uniqueTestsPassed'],'unique tests; guarded native qualification complete')
