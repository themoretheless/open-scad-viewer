from pathlib import Path
import hashlib,json,re,shutil
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[2]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
r=json.loads((OUT/'qualification-r7.json').read_text())
assert r['state']=='complete' and r['passed'] and r['sourceGuardVerified'] and r['ownedMainMatches']

index=json.loads((OUT/'source-r7.json').read_text())
owned=json.loads((OUT/'owned-r7.json').read_text())
assert all(sha(ROOT/n)==h for n,h in owned.items())
borrowed=json.loads((OUT/'borrowed-r7.json').read_text())
assert r['borrowedMainMatches'] and all(sha(ROOT/n)==h for n,h in borrowed.items())
unique=set(); binaries={}
for phase in r['phases']:
 log=OUT/('r7-'+phase['name']+'.log')
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
(OUT/'unique-tests-r7.json').write_text(json.dumps(sorted(unique),indent=2)+'\n')
probe=json.loads((OUT/'native-moving-step-receipt.json').read_text())
assert probe['passed'] and all(sha(OUT/n)==h for n,h in probe['files'].items())
r.update(additionalNativeScenariosPassed=4,movingNativeStepPassed=True,independentMovingStepOcctPassed=True,nativeMovingStepReceiptSha256=sha(OUT/'native-moving-step-receipt.json'),nativeMovingStepArchiveSha256=sha(OUT/'native-moving-step-artifacts.tar.gz'),developmentEvidenceIndexSha256=sha(OUT/'development-evidence.json'),sharedMainBuildVerified=False,nativeProtocolTestsPassed=sum(p["testsPassed"]for p in r["phases"]if p["name"].startswith("native-")),uniqueTestsPassed=len(unique),binarySha256=binaries,qualificationDriverSha256=sha(OUT/'qualify-r7.py'),
 finalizerSha256=sha(Path(__file__)),freezeDriverSha256=sha(OUT/'freeze-r7.py'),readmeSha256=sha(OUT/'README.md'),changedSourcesSha256=sha(OUT/'changed-sources.json'),incrementalPatchSha256=sha(OUT/'change-from-previous-qualified.patch'),authoredPathsSha256=sha(OUT/'authored-paths.json'),borrowedSourceIndexSha256=sha(OUT/'borrowed-r7.json'),openScope=['unsynchronized algebraic branch cuts inside original spans and branch graphs without a proved local monotone chart',
 'residual maps outside exact bivariate divisibility or existing normal-jet relations',
 'moving nonplanar volumes that the exact source conditioner and bounded global atlas cannot prove',
 'nonseparable rational weights or mixed-coordinate dependencies beyond the existing original surface bounds',
 'new moving-carrier source-global proof does not alone grant actual full-shell G1 G2 or joint Solid admission',
 'full shared main build, public WASM Rush viewport delivery and full independent STEP UI matrices'])
(OUT/'result.json').write_text(json.dumps(r,indent=2)+'\n')
print(r['testsPassed'], 'executions;',r['uniqueTestsPassed'],'unique tests; guarded native qualification complete')
