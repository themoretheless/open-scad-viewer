from pathlib import Path
import hashlib,json,tarfile,re
OUT=Path(__file__).resolve().parent
STAGE=Path('/private/tmp/osv-sweep-independent-20261030-r1')
ROOT=OUT.parents[2]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
r=json.loads((OUT/'qualification-r4.json').read_text())
assert r['state']=='complete' and r['passed'] and r['sourceGuardVerified'] and r['ownedMainMatches']
assert sha(OUT/'source-r4.json')==r['sourceIndexSha256']
assert sha(OUT/'qualified-source-r4.tar.gz')==r['sourceArchiveSha256']
for phase in r['phases']:
 assert phase['state']=='passed' and phase['returncode']==0
 assert sha(OUT/('r4-'+phase['name']+'.log'))==phase['logSha256']
for n,h in r['ownedSources'].items():assert sha(ROOT/n)==h
core=json.loads((OUT/'source-r4.json').read_text())
extra=json.loads((OUT/'extra-fixtures-r4.json').read_text())
assert all(sha(STAGE/n)==h for n,h in {**core,**extra}.items())
with tarfile.open(OUT/'qualified-extra-fixtures-r4.tar.gz','w:gz',compresslevel=3)as tar:
 for n in extra:tar.add(STAGE/n,arcname=n,recursive=False)
passed=[]
for phase in r['phases']:
 artifact=None
 for line in (OUT/('r4-'+phase['name']+'.log')).read_text().splitlines():
  run=re.search(r'Running .*\(([^)]+)\)',line)
  if run:artifact=Path(run.group(1)).name
  test=re.fullmatch(r'test (.+) \.\.\. ok',line)
  if test:
   assert artifact is not None
   passed.append((artifact,test.group(1)))
assert len(passed)==r['testsPassed']
unique=sorted(set(passed))
(OUT/'unique-passed-tests.json').write_text(json.dumps(unique,indent=2)+'\n')
result={
 'schema':'sweep-independent-original-bases/1','state':'complete','passed':True,
 'testsPassed':r['testsPassed'],'uniqueTestsPassed':len(unique),'testCountIncludesRepeatedFilters':True,
 'uniqueTestIndexSha256':sha(OUT/'unique-passed-tests.json'),'fullGoalComplete':False,
 'limitsIncreased':False,'wasmPublished':False,'uiOrFocusInteraction':False,
 'sourceGuardVerified':True,'ownedMainMatches':True,
 'qualificationReceipt':'qualification-r4.json','qualificationReceiptSha256':sha(OUT/'qualification-r4.json'),
 'sourceIndex':'source-r4.json','sourceIndexSha256':r['sourceIndexSha256'],
 'sourceArchive':'qualified-source-r4.tar.gz','sourceArchiveSha256':r['sourceArchiveSha256'],
 'additionalInputsIndex':'extra-fixtures-r4.json','additionalInputsIndexSha256':sha(OUT/'extra-fixtures-r4.json'),
 'additionalInputsArchive':'qualified-extra-fixtures-r4.tar.gz','additionalInputsArchiveSha256':sha(OUT/'qualified-extra-fixtures-r4.tar.gz'),
 'qualifiedOwnedSources':r['ownedSources'],
 'implemented':[
  'Exact original-span G1/G2 comparisons with independent along degrees, control counts and knot partitions',
  'Positive fractional-linear seam proposals from original interior knots with exact whole-span identities',
  'Exact affine world-curve/support binding across independent bases; linear pcurves cannot inherit fractional-linear parameter maps',
  'Clamped closed world curves can own periodic support traces only after exact original endpoint closure',
  'Charged unresolved decisions preserve rounded-reversal refusal and do not refund failed work'],
 'validatedCaseFamilies':['nonuniform','nonclamped','rational degree elevation and weight gauges','periodic source joins','fractional-linear source knot maps','full doubly periodic body with independent cubic world edge','interior mismatch and one-ULP mutation refusals','zero and one-short exact budgets'],
 'auditedGates':['original source laws/frames/RMF/holonomy','retained walls/decomposition/filled caps','complete boundary G1/G2','embedding and regularity','material volume and orientation','shell nesting','self intersections'],
 'remaining':[
  'Arbitrary nonlinear seam reparameterizations and unrelated projective gauges',
  'A complete all-declared-combination continuousBound proof is not established by this finite native regression audit',
  'Unrestricted global intersection, nesting, orientation and whole-boundary smoothness coverage remains conditional on existing proof owners'],
 'finite207CaseMatrixReplayed':False,
 'noCodecScope':'cad-predicates and nurbs-core only; no new BRep no-codec claim',
 'readme':'README.md','readmeSha256':sha(OUT/'README.md'),
 'sourceGateAudit':'source-gate-audit.json','sourceGateAuditSha256':sha(OUT/'source-gate-audit.json'),
 'driverSha256':sha(OUT/'qualify-r4.py'),'finalizerSha256':sha(__file__)
}
(OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'state':result['state'],'testsPassed':result['testsPassed'],'fullGoalComplete':False}))
