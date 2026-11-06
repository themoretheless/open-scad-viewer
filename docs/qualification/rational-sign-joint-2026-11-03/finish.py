from pathlib import Path
import json,hashlib,re,tarfile,subprocess
Q=Path(__file__).resolve().parent;ROOT=Q.parents[2];S=Path('/private/tmp/osv-rational-sign-20261103')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
r=json.loads((Q/'audit.json').read_text())
assert r['state']=='complete' and r['passed'] and r['testsFailed']==0 and len(r['phases'])==11
for p in r['phases']:assert p['returncode']==0 and sha(Q/(p['name']+'.log'))==p['logSha256']
ledger=json.loads((Q/'source.json').read_text());owned=json.loads((Q/'owned.json').read_text())
assert all(sha(S/n)==h for n,h in ledger.items())
assert all(sha(ROOT/n)==h for n,h in owned.items())
assert all(sha(Q/n)==h for n,h in json.loads((Q/'inputs.json').read_text()).items())
from fractions import Fraction as F
for case in ['body','body-both']:
 for name in ['boundary','imported','transport']:
  a=json.loads((Q/f'{case}/{name}.json').read_text())
  assert a['certified'] and a['continuousBound'] and a['withinBudget'] and a['errorUpper']==0
  assert a['referenceRegularity']['proven'] and a['retainedRegularity']['proven']
  assert not a['smoothnessCertified'] and not a['sourceMotionLawCertified']
  for k in ['referenceVolume','retainedVolume']:
   for flag in ['proven','allFacesInjective','allPairsClassified','exactBoundary','trimRegionsValid','nestingConsistent']:assert a[k][flag],(case,name,k,flag)
   assert all(v['outward']==v['expectedOutward']for v in a[k]['orientations'])
 for name in ['original','imported','transport']:
  a=json.loads((Q/f'{case}/short-{name}.json').read_text())
  assert not a['certified'] and a['continuousBound'] and a['errorUpper']==0
 for name in ['station','imported-station']:
  a=json.loads((Q/f'{case}/{name}.json').read_text());assert a['complete']and a['g1']and a['g2']and len(a['edges'])==3
 o=json.loads((Q/f'{case}/independent-occt.json').read_text())
 e=json.loads((Q/f'{case}/exact-oracle.json').read_text())
 assert o['passed']and o['stepSha256']==sha(Q/f'{case}/body.step')and o['integrationEps']==1e-10 and o['absoluteTolerance']==1e-7
 assert e['passed']and e['nativeBodySha256']==sha(Q/f'{case}/body.json')and e['exactGreenIntegralEnclosed']
 assert 0<F(e['volumeUpperFraction'])-F(e['volumeLowerFraction'])<F(1,10**30)
 c=json.loads((Q/f'{case}/charts.json').read_text());assert c['proven']and c['work']<=1000 and all(f['proven']for f in c['faces'])
 j=json.loads((Q/f'{case}/transverse-jets.json').read_text());assert j['c2AtWitness']==(case=='body')
 if case=='body-both':assert j['leftDvv']==[0,0,10]and j['rightDvv']==[0,0,-10]
r.update(sourceManifestSha256=sha(Q/'source.json'),ownedManifestSha256=sha(Q/'owned.json'),
 baseSnapshot='rational-product-joint-2026-11-03',baseManifestSha256=sha(ROOT/'docs/qualification/rational-product-joint-2026-11-03/source.json'),
 nonMobiusRationalCoordinateSignQualified=True,wholeBoundaryGlobalAndActualStationG2Qualified=True,
 nativeGeometryCases=2,stepJointCertificates=6,globalBudgetRefusals=6,actualInternalStationG2SeamsPerCase=3,
 sourceChartWork={case:json.loads((Q/f'{case}/charts.json').read_text())['work']for case in ['body','body-both']},
 externalOpenCascadePassed=True,exactRationalVolumeEnclosurePassed=True,
 occtRequestedIntegrationEpsMet={case:json.loads((Q/f'{case}/independent-occt.json').read_text())['integrationRequestedEpsMet']for case in ['body','body-both']},
 occtErrorEstimateControlsAcceptance=False,
 transverseC2JumpDoesNotInvalidateProvedG2=True,fullShellSmoothnessCertified=False,
 arbitraryRationalMapCompleteness=False,currentConcurrentHeadQualified=False,fullGoalComplete=False,uiChecked=False,wasmPublished=False)
r['preexistingIgnoredMeasurements']={name:re.findall(r'^test (.+?) \.\.\. ignored.*$',(Q/(name+'.log')).read_text(),re.M)for name in ['nurbs-full','brep-full']}
r['toolSourceHashes']={n:sha(Q/n)for n in ['qualify.py','finish.py','probe/Cargo.toml','probe/Cargo.lock','probe/src/main.rs','exact_oracle.py','verify-occt.py']}
r['compiler']=subprocess.check_output(['rustc','-Vv'],cwd=S,text=True)
(Q/'audit.json').write_text(json.dumps(r,indent=2)+'\n')
p=Q/'README.md';text=p.read_text().split('\n## Final results')[0]
text=text.replace('Qualification in progress. Native Rust only; WASM/UI are outside this run.',f'Qualification completed on the frozen {len(ledger)}-file source snapshot. Native Rust only; no WASM/UI publication in this run.')
text+='\n## Final results\n\n| Gate | Passed | Failed | Ignored |\n| --- | ---: | ---: | ---: |\n'
for g in r['phases']:
 if g['testsPassed'] or g['testsFailed'] or g['testsIgnored']:text+=f"| {g['name']} | {g['testsPassed']} | {g['testsFailed']} | {g['testsIgnored']} |\n"
text+='\nCounts are executions per gate. Existing ignored measurement cases remain named in `audit.json`; no new exclusions were introduced. The focused five new joint tests are also included in the full BRep gate.\n'
text+=f"\nBoth native bodies have fresh original, imported and transported certificates: six complete continuous-bound/global-volume certificates and six low-global-budget refusals. Every positive boundary independently proves regularity, injection, trim, contact, nesting and orientation. Each body has three actual station G1/G2 seams before and after STEP. The two-clock case has a real transverse C2 jump (+10/-10) while exact geometric G2 holds. Full-shell sharp smoothness remains false. Source chart work is {r['sourceChartWork']} under the unchanged shared 1000-unit limit.\n"
text+="\nBoth exact rational volume enclosures have width below 1e-30 around 141.91438726444202655. Independent OCCT accepts two valid 1-solid/8-face/15-edge/9-vertex STEP bodies, with zero interference pairs and 56 exact-parameter boundary checks each. The adaptive Gauss-Kronrod kernel owner uses the former Eps=1e-10 and acceptance tolerance 1e-7. The standard Gauss owner underestimates its error on the new rational trim; its disagreement is preserved in diagnostic fields, together with the initial failed external run. The two-clock GK run does not meet its requested internal Eps estimate; this remains explicitly reported and is not called convergence. The actual mass comparison uses the exact source-volume enclosure and the unchanged absolute 1e-7 tolerance. No acceptance tolerance was raised. Closest-point diagnostics do not control acceptance.\n"
text+="\nThe initial naive derivative-product proof exhausted the shared face ledger, although local maps were proved. Its refusal is preserved in `development-body/`. Antisymmetric coefficient pairs and exact positive-factor cancellation close the budget instead of raising it. The intermediate two-clock budget refusal is preserved in `development-both/`. Source weights/caps were not simplified to obtain these final certificates.\n"
text+='\nThe orchestration checkpoint after NURBS was refreshed when independent Python diagnostics were updated. Its hash guard stopped before BRep, then resumed using the same verified Rust snapshot and retained NURBS log; no Rust sources, input bodies or proof budgets changed.\n'
text+='\nFrozen source, selected owned main files, probe inputs and every gate log were hash-verified after qualification. `audited-source.tar.gz` includes the baseline literal compile-time inputs plus this overlay. `source.json`, `owned.json`, `inputs.json`, `audit.json` and `evidence.tar.gz` record reproducibility. Concurrent unrelated main edits are outside this source qualification.\n'
p.write_text(text)
with tarfile.open(Q/'evidence.tar.gz','w:gz',compresslevel=3)as a:
 for p in sorted(Q.rglob('*')):
  if p.is_file() and not p.name.endswith('.tar.gz'):a.add(p,arcname=str(p.relative_to(Q)),recursive=False)
print(json.dumps(dict(passed=True,testsPassed=r['testsPassed'],testsFailed=0,fullGoalComplete=False)))
