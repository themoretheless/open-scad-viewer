from pathlib import Path
import json,hashlib,re,subprocess,tarfile
Q=Path(__file__).resolve().parent;ROOT=Q.parents[2];S=Path('/private/tmp/osv-quintic-global-20261103')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
r=json.loads((Q/'audit.json').read_text());assert r['state']=='complete' and r['passed'] and r['testsFailed']==0
assert len(r['phases'])==10
for p in r['phases']:assert p['returncode']==0 and sha(Q/(p['name']+'.log'))==p['logSha256']
ledger=json.loads((Q/'source.json').read_text());owned=json.loads((Q/'owned.json').read_text())
assert all(sha(S/n)==h for n,h in ledger.items());assert all(sha(ROOT/n)==h for n,h in owned.items())
assert all(sha(Q/n)==h for n,h in json.loads((Q/'inputs.json').read_text()).items())
for name in ['original','imported','transport']:
 a=json.loads((Q/f'affine-body/{name}.json').read_text());b=json.loads((Q/f'affine-body/short-{name}.json').read_text())
 assert a['certified'] and a['continuousBound'] and a['withinBudget'] and a['errorUpper']==0
 assert a['referenceRegularity']['proven'] and a['retainedRegularity']['proven']
 assert not a['smoothnessCertified'] and not a['sourceMotionLawCertified']
 for k in ['referenceVolume','retainedVolume']:
  for flag in ['proven','allFacesInjective','allPairsClassified','exactBoundary','trimRegionsValid','nestingConsistent']:assert a[k][flag],(name,k,flag)
  assert all(v['outward']==v['expectedOutward'] for v in a[k]['orientations'])
 assert not b['certified'] and b['continuousBound'] and b['errorUpper']==0
 assert b['referenceRegularity']['proven'] and b['retainedRegularity']['proven']
t=json.loads((Q/'affine-body/transport.json').read_text());assert t['uvTransportFaces']==[2]
o=json.loads((Q/'affine-body/independent-occt.json').read_text());assert o['passed'] and o['stepSha256']==sha(Q/'affine-body/quintic-body.step')
e=json.loads((Q/'exact-oracle.json').read_text());assert e['passed'] and e['nativeBodySha256']==sha(Q/'affine-body/body.json')
assert e['analyticVolume']=='800/7' and e['affineDeterminant']=='1' and e['allCarrierAndCapTrimsBound'] and e['exactInverseReconstruction']
s=json.loads((Q/'affine-body/smoothness.json').read_text());assert not s['fullBoundaryG1Certified'] and not s['fullBoundaryG2Certified']
c=json.loads((Q/'affine-body/chart.json').read_text());assert c['proven'] and c['work']<1000 and c['contractionUpper'] is None
assert c['reason']=='global-exact-affine-separable-monotonicity'
r.update(frozenFiles=len(ledger),sourceArchiveSha256=sha(Q/'audited-source.tar.gz'),sourceManifestSha256=sha(Q/'source.json'),compileInputCaptureSha256=sha(Q/'compile-input-capture.json'),originalQuinticGlobalCertificateConfirmed=True,affineMixedQuinticGlobalCertificate=True,stepJointCertificates=3,globalBudgetRefusals=3,sourceChartWork=c['work'],sourceRegularityCells=t['referenceRegularity']['cells'],retainedRegularityCells=t['retainedRegularity']['cells'],externalOpenCascadePassed=True,exactPolynomialOraclePassed=True,fullGoalComplete=False,arbitraryAlgebraicMapCompleteness=False,uiChecked=False,wasmPublished=False)
ignored={}
for name in ['nurbs-full','brep-full']:
 ignored[name]=re.findall(r'^test (.+?) \.\.\. ignored.*$',(Q/(name+'.log')).read_text(),re.M)
r['preexistingIgnoredMeasurements']=ignored
r['toolSourceHashes']={n:sha(Q/n)for n in ['qualify.py','finish.py','probe/Cargo.toml','probe/Cargo.lock','probe/src/main.rs','exact_oracle.py','verify-affine-occt.py']}
r['compiler']=subprocess.check_output(['rustc','-Vv'],cwd=S,text=True)
r['cargo']=subprocess.check_output(['cargo','-V'],cwd=S,text=True).strip()
(Q/'audit.json').write_text(json.dumps(r,indent=2)+'\n')
p=Q/'README.md';text=p.read_text().split('\n## Final results')[0].replace('Qualification is in progress.',f'Qualification completed for the frozen {len(ledger)}-file Rust source snapshot.')
text+='\n## Final results\n\n| Gate | Passed | Failed | Ignored |\n| --- | ---: | ---: | ---: |\n'
for g in r['phases']:
 if g['testsPassed'] or g['testsFailed'] or g['testsIgnored']:text+=f"| {g['name']} | {g['testsPassed']} | {g['testsFailed']} | {g['testsIgnored']} |\n"
text+='\nCounts are test executions per gate; the focused critical tests are also included in the full BRep suite. Existing ignored cases remain measurement/benchmark diagnostics, with names recorded in `audit.json`; no new exclusions were introduced.\n'
text+=f"\nThe new mixed chart uses {c['work']} of the former 1000 source work units. Source/retained regularity uses {t['referenceRegularity']['cells']} / {t['retainedRegularity']['cells']} cells under the unchanged shared 10000-cell boundary limit. The earlier exhausted-normal-cover attempt is preserved in `development-regularity-exhausted/`.\n"
text+=f"\nFresh original, imported and transported boundaries all have continuous error upper bound 0 AND complete independent global certificates. Three insufficient-global-budget controls preserve continuous error 0 while refusing the global certificate, despite caller success flags. Genuine rank-loss and missing source ownership also refuse. Sharp full-body G1/G2 remain false.\n\nThe exact rational oracle binds all carriers, complete cap trims, source polynomial identities, the inverse affine map and determinant 1 to the fresh body SHA-256; volume is exactly 800/7. Independent OCCT reports a valid 1-solid/4-face/6-edge/4-vertex body, 28 exact-parameter boundary checks, 15 material classifications, zero interference pairs and absolute volume error {o['absoluteVolumeError']} under the unchanged 1e-7 tolerance. Closest-point diagnostics do not control acceptance.\n"
text+='\nThe source archive includes all literal compile-time imports. Additional immutable inputs were checked against base Git blobs and recorded in `compile-input-capture.json`; no Rust or fixture contents changed for this metadata completion.\n'
text+='\nFrozen source, selected owned-main files, model input and phase-log hashes were verified after all runs. Newer concurrent main changes and test targets outside this snapshot are not qualified here. The missing-target launch error was preserved as `launch-attempt-algebraic-maps.log`; the selected frozen carrier/coupled-graph matrix then passed without changing Rust sources or limits.\n'
text+='\n## Remaining scope\n\nThe ordinary quintic global block is closed in native Rust; the exact affine-mixed variant is now also closed through STEP. Mathematical completeness for arbitrary algebraic maps and for all combinations of source laws, frames, caps and closed seams with applicable G1/G2 remains open. Rational/nonpolynomial cases outside the proved classes remain unsupported. Correct unsupported/budget refusals are retained. A retained-boundary certificate does not certify an ideal sweep motion law. This run does not publish or qualify WASM/UI.\n\n`source.json`, `owned.json`, `audit.json`, `audited-source.tar.gz` and `evidence.tar.gz` are the reproducibility records.\n'
p.write_text(text)
with tarfile.open(Q/'evidence.tar.gz','w:gz',compresslevel=3)as archive:
 for p in sorted(Q.rglob('*')):
  if p.is_file() and not p.name.endswith('.tar.gz'):archive.add(p,arcname=str(p.relative_to(Q)),recursive=False)
print(json.dumps({'passed':r['passed'],'testExecutionsPassed':r['testsPassed'],'failed':r['testsFailed'],'fullGoalComplete':False}))
