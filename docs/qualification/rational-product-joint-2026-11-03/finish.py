from pathlib import Path
import json,hashlib,re,tarfile,subprocess
Q=Path(__file__).resolve().parent;ROOT=Q.parents[2];S=Path('/private/tmp/osv-rational-product-20261103')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
r=json.loads((Q/'audit.json').read_text())
assert r['state']=='complete' and r['passed'] and r['testsFailed']==0 and len(r['phases'])==8
for p in r['phases']:assert p['returncode']==0 and sha(Q/(p['name']+'.log'))==p['logSha256']
ledger=json.loads((Q/'source.json').read_text());owned=json.loads((Q/'owned.json').read_text())
assert all(sha(S/n)==h for n,h in ledger.items())
assert all(sha(ROOT/n)==h for n,h in owned.items())
assert all(sha(Q/n)==h for n,h in json.loads((Q/'inputs.json').read_text()).items())
for name in ['boundary','imported','transport']:
 a=json.loads((Q/f'body/{name}.json').read_text())
 assert a['certified'] and a['continuousBound'] and a['withinBudget'] and a['errorUpper']==0
 assert a['referenceRegularity']['proven'] and a['retainedRegularity']['proven']
 assert not a['smoothnessCertified'] and not a['sourceMotionLawCertified']
 for k in ['referenceVolume','retainedVolume']:
  for flag in ['proven','allFacesInjective','allPairsClassified','exactBoundary','trimRegionsValid','nestingConsistent']:assert a[k][flag],(name,k,flag)
  assert all(v['outward']==v['expectedOutward'] for v in a[k]['orientations'])
for name in ['original','imported','transport']:
 a=json.loads((Q/f'body/short-{name}.json').read_text())
 assert not a['certified'] and a['continuousBound'] and a['errorUpper']==0
for name in ['station','imported-station']:
 a=json.loads((Q/f'body/{name}.json').read_text())
 assert a['complete'] and a['g1'] and a['g2'] and len(a['edges'])==3
o=json.loads((Q/'body/independent-occt.json').read_text())
e=json.loads((Q/'body/exact-oracle.json').read_text())
assert o['passed'] and o['stepSha256']==sha(Q/'body/body.step')
assert e['passed'] and e['nativeBodySha256']==sha(Q/'body/body.json') and e['analyticVolume']=='800/7'
r.update(sourceManifestSha256=sha(Q/'source.json'),ownedManifestSha256=sha(Q/'owned.json'),
 baseSnapshot='quintic-global-2026-11-03',baseManifestSha256=sha(ROOT/'docs/qualification/quintic-global-2026-11-03/source.json'),
 rationalProductClocksQualified=True,wholeBoundaryGlobalAndStationG2Qualified=True,
 stepJointCertificates=3,globalBudgetRefusals=3,actualInternalStationG2Seams=3,
 externalOpenCascadePassed=True,exactRationalOraclePassed=True,
 fullShellSmoothnessCertified=False,arbitraryRationalMapCompleteness=False,
 currentConcurrentHeadQualified=False,fullGoalComplete=False,uiChecked=False,wasmPublished=False)
r['preexistingIgnoredMeasurements']={name:re.findall(r'^test (.+?) \.\.\. ignored.*$',(Q/(name+'.log')).read_text(),re.M)for name in ['nurbs-full','brep-full']}
r['toolSourceHashes']={n:sha(Q/n)for n in ['qualify.py','finish.py','probe/Cargo.toml','probe/Cargo.lock','probe/src/main.rs','exact_oracle.py','verify-occt.py']}
r['compiler']=subprocess.check_output(['rustc','-Vv'],cwd=S,text=True)
(Q/'audit.json').write_text(json.dumps(r,indent=2)+'\n')
p=Q/'README.md';text=p.read_text().split('\n## Final results')[0]
text=text.replace('Qualification in progress. Native Rust only; no WASM/UI publication in this run.',f'Qualification completed on the frozen {len(ledger)}-file source snapshot. Native Rust only; no WASM/UI publication in this run.')
text+='\n## Final results\n\n| Gate | Passed | Failed | Ignored |\n| --- | ---: | ---: | ---: |\n'
for g in r['phases']:
 if g['testsPassed'] or g['testsFailed'] or g['testsIgnored']:text+=f"| {g['name']} | {g['testsPassed']} | {g['testsFailed']} | {g['testsIgnored']} |\n"
text+='\nCounts are executions per gate. Existing ignored measurement cases remain named in `audit.json`; no new exclusions were introduced. The focused three new joint tests are also included in the full BRep gate.\n'
text+=f"\nOriginal, imported and transported boundaries all prove continuous error zero together with complete regularity, chart injection, trim, contact, nesting and orientation. All three actual station seams independently prove G1/G2 before and after STEP. Three low-global-budget controls retain the zero continuous bound but refuse Solid even when caller success fields are forged. Separate Rust tests preserve rank-loss, folded-map, weight-drift and missing topology refusals, plus exact-budget replay and one-short failure. Sharp full-shell smoothness remains false.\n\nThe exact rational oracle reconstructs the denominator and both coordinate numerators on the full interval, including the non-dyadic critical parameter 2/3. Independent OCCT reports a valid 1-solid/8-face/15-edge/9-vertex body, zero interference pairs, and absolute volume error {o['absoluteVolumeError']} under the unchanged 1e-7 tolerance. Exact-parameter boundary point error is {o['maxParameterPointError']}; closest-point diagnostics do not control acceptance.\n"
text+='\nFrozen source, selected owned main files, probe inputs and every gate log were hash-verified after qualification. `audited-source.tar.gz` includes the baseline literal compile-time inputs plus this overlay. `source.json`, `owned.json`, `inputs.json`, `audit.json` and `evidence.tar.gz` record reproducibility. Concurrent unrelated main edits are outside this source qualification.\n'
p.write_text(text)
with tarfile.open(Q/'evidence.tar.gz','w:gz',compresslevel=3)as a:
 for p in sorted(Q.rglob('*')):
  if p.is_file() and not p.name.endswith('.tar.gz'):a.add(p,arcname=str(p.relative_to(Q)),recursive=False)
print(json.dumps(dict(passed=True,testsPassed=r['testsPassed'],testsFailed=0,fullGoalComplete=False)))
