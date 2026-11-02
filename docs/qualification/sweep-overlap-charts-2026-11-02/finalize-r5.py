from pathlib import Path
import hashlib,json,re
Q=Path(__file__).resolve().parent
ROOT=Q.parents[2]
STAGE=Path('/private/tmp/osv-sweep-overlap-20261102-r5')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
r=json.loads((Q/'qualification-r5.json').read_text())
assert r['passed'] and r['state']=='complete'
ledger=json.loads((Q/'source-r5.json').read_text())
changed=json.loads((Q/'changed-r5.json').read_text())
assert all(sha(STAGE/n)==h for n,h in ledger.items())
assert all(sha(ROOT/n)==h for n,h in changed.items())
assert sha(Q/'qualified-source-r5.tar.gz')==r['sourceArchiveSha256']
unique=set();executables={}
scopes={'predicates':'cad-predicates/lib','continuity':'nurbs-core/lib','world-traces':'nurbs-core/lib','brep-sweep-audit':'brep-core/lib','native-complete':'geometry-bridge','no-codec':'check'}
for phase in r['phases']:
 log=Q/('r5-'+phase['name']+'.log');assert sha(log)==phase['logSha256']
 scope=scopes[phase['name']]
 for line in log.read_text().splitlines():
  match=re.search(r'Running tests/(\S+)\.rs \(([^\n]+)\)',line)
  if match:scope='geometry-bridge/'+match[1]
  match=re.search(r'Running (?:unittests|tests/)?.*\((/private/tmp/[^\n]+)\)',line)
  if match:
   p=Path(match[1]);assert p.is_file();executables[str(p)]=sha(p)
  match=re.fullmatch(r'test (.+) \.\.\. ok',line)
  if match:unique.add(scope+'::'+match[1])
assert r['testsPassed']==len(unique)
assert r['testsPassed']==345
p=Q/'README.md';s=p.read_text().replace('Qualification in progress.','Qualified: 345 tests passed (110 predicates, 63 continuity, 25 world traces, 119 sweep/BRep, 28 native protocol tests), plus a no-codec build.')
s+='\nThe native affine-tilted arc and arc-clock G2 tests freshly export STEP, import it and recompute certificates. These are Rust round-trip checks; no new independent OpenCascade matrix is claimed.\n'
s+='\nFinal snapshot: 3291 source files and nine changed main paths, all hash-verified. The source archive and incremental patch preserve the exact native qualification input. No fresh external STEP/UI matrix or WASM publication is included.\n'
p.write_text(s)
r.update(uniqueTestsPassed=len(unique),nativeProtocolTestsPassed=28,sourceFileCount=len(ledger),changedSourceCount=len(changed),executables=executables,readmeSha256=sha(p),incrementalPatchSha256=sha(Q/'change-from-interior-cuts.patch'),developmentEvidenceSha256=sha(Q/'development-evidence.json'),qualificationScriptSha256=sha(Q/'qualify-r5.py'),finalizerSha256=sha(Path(__file__)),freshExternalStepQualified=False,nativeStepRoundTripTestsPassed=2,fullGoalComplete=False,sharedMainBuildVerified=False,openScope=['Algebraic components without a proved common factor and strict local branch exclusion','Singular or ambiguous maps outside the original regular normal-jet class','Shifted whole-loop phase without a proved common start point','Full dirty shared-main/WASM/Rush/viewport delivery and complete fresh independent STEP/UI matrices'])
(Q/'unique-tests-r5.json').write_text(json.dumps(sorted(unique),indent=2)+'\n')
r['uniqueTestIndexSha256']=sha(Q/'unique-tests-r5.json')
(Q/'result.json').write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps({k:r[k]for k in ['passed','testsPassed','uniqueTestsPassed','nativeProtocolTestsPassed','sourceGuardVerified','changedMainMatches','limitsIncreased','uiOrFocusInteraction','wasmPublished','fullGoalComplete']},indent=2))
