from pathlib import Path
import json,hashlib,tarfile
Q=Path(__file__).resolve().parent;ROOT=Q.parents[2];STAGE=Path('/private/tmp/osv-sweep-followup-20261102')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
a=json.loads((Q/'audit.json').read_text());assert a['state']=='complete' and a['passed'] and a['testsFailed']==0
assert len(a['phases'])==11 and all(p['returncode']==0 for p in a['phases'])
l=json.loads((Q/'source-final.json').read_text());assert all(sha(STAGE/p)==h for p,h in l.items())
for p in a['phases']:assert sha(Q/(p['name']+'.log'))==p['logSha256']
q=Q/'native-body';o=json.loads((q/'independent-occt.json').read_text());assert o['passed'] and o['stepSha256']==sha(q/'quintic-body.step')
r=json.loads((q/'boundary-transport.json').read_text())
assert r['continuousBound'] and r['errorUpper']==0 and r['uvTransportFaces']==[2]
assert not r['certified'] and r['reason']=='solid-geometry-unproved'
for k in ['referenceVolume','retainedVolume']:
 assert r[k]['trimRegionsValid'] and r[k]['exactBoundary'] and not r[k]['allFacesInjective'] and not r[k]['proven']
for k in ['referenceRegularity','retainedRegularity']:assert r[k]['proven']
s=json.loads((q/'smoothness.json').read_text());assert not s['fullBoundaryG1Certified'] and not s['fullBoundaryG2Certified']
p='crates/brep-core/src/sweep_boundary_owned/closed/capped_material/tests/rational_hollow.rs'
assert (STAGE/p).read_bytes()==(Path('/private/tmp/osv-sweep-gram-20261102-r4')/p).read_bytes()==(ROOT/p).read_bytes()
a.update(independentOpenCascadePassed=True,quinticCapContoursProven=True,originalToStepContinuousBound=True,originalToStepErrorUpper=0.,quinticStrictGlobalCertificate=False,sharpBoundaryG1Certified=False,sharpBoundaryG2Certified=False,limitsRaised=False,quarterTurnRegressionUnchanged=True,fullGoalComplete=False,wasmPublished=False,uiChecked=False)
(Q/'audit.json').write_text(json.dumps(a,indent=2)+'\n')
p=Q/'README.md';text=p.read_text().replace('Qualification is in progress. No all-mode or full mathematical coverage claim.','Qualification completed for the frozen Rust snapshot. No all-mode or full mathematical coverage claim.')
text+='\n## Final measured results\n\n'
text+=f"- {a['testsPassed']} tests passed; {a['testsFailed']} failed across the 11 audit phases; no-codec check passed. The five original failing cases are included unchanged in the full sweep gate.\n"
text+='- New fixed-metric complete-cover P-matrix tests include exact-budget replay, one-cell-short refusal and an interior fold; all seven principal minors are checked.\n'
text+='- Both quintic cap contours are proven simple in original and STEP-imported UV charts. Original/retained boundary correspondence is proven with error upper bound 0 and exact U transport of face 2. Regularity is independently proven.\n'
text+=f"- Fresh independent OpenCascade: valid 1-solid/4-face/6-edge/4-vertex STEP; absolute volume error {o['absoluteVolumeError']}; 28 exact-parameter point checks, 15 material classifications, no interference pairs at level 5. Absolute tolerance remains 1e-7. The closest-point diagnostic does not control acceptance.\n"
text+='- The quintic strict global certificate still refuses: all-face injectivity, all pair classifications, material nesting and shell orientation are not yet jointly established. Simple contours and valid OCCT geometry do not supply those proofs. Whole sharp-body G1/G2 remains false; the intended joins are C0.\n'
text+='- General algebraic maps and all joint continuousBound/global/applicable-G1/G2 combinations remain open. The new volume theorem admits one smooth nonperiodic rational chart, not arbitrary periodic or C0 atlases.\n'
text+='- Frozen-source guard and owned-main hashes passed; unrelated shared work was preserved. WASM was not republished; UI was not exercised and no focus was requested.\n'
text+='\n`audit.json` is the phase-level source of truth. Failed development attempts are preserved but excluded from these final counts. `evidence.tar.gz` retains the current qualification artifacts.\n'
p.write_text(text)
with tarfile.open(Q/'evidence.tar.gz','w:gz',compresslevel=3)as t:
 for p in sorted(Q.rglob('*')):
  if p.is_file() and p.name!='evidence.tar.gz' and p.suffix not in ['.gz']:
   t.add(p,arcname=str(p.relative_to(Q)),recursive=False)
print(json.dumps({k:a[k]for k in ['testsPassed','testsFailed','sourceGuardVerified','changedMainMatches','fullGoalComplete']}))
