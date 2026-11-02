from pathlib import Path
import hashlib,json,re,tarfile
OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[2]
STAGE=Path('/private/tmp/osv-sweep-nonlinear-joint-20261030-r9')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
r=json.loads((OUT/'qualification-r9.json').read_text())
assert r['passed'] and r['state']=='complete' and r['sourceGuardVerified'] and r['ownedMainMatches']
ledger=json.loads((OUT/'source-r9.json').read_text())
assert sha(OUT/'source-r9.json')==r['sourceIndexSha256']
assert sha(OUT/'qualified-source-r9.tar.gz')==r['sourceArchiveSha256']
assert all(sha(STAGE/n)==h for n,h in ledger.items())
assert all(sha(ROOT/n)==h for n,h in r['ownedSources'].items())
passed=[];binaries={}
for phase in r['phases']:
 assert phase['state']=='passed' and phase['returncode']==0
 log=OUT/('r9-'+phase['name']+'.log');assert sha(log)==phase['logSha256']
 artifact=None
 for line in log.read_text().splitlines():
  run=re.search(r'Running .*\(([^)]+)\)',line)
  if run:
   artifact=Path(run.group(1)).name;p=Path(run.group(1))
   if p.is_file():binaries[str(p)]=sha(p)
  test=re.fullmatch(r'test (.+) \.\.\. ok',line)
  if test:
   assert artifact is not None;passed.append((artifact,test.group(1)))
assert len(passed)==r['testsPassed']
unique=sorted(set(passed))
(OUT/'unique-passed-tests.json').write_text(json.dumps(unique,indent=2)+'\n')
result={
 'schema':'sweep-nonlinear-joint/1','state':'complete','passed':True,'fullGoalComplete':False,
 'testsPassed':r['testsPassed'],'uniqueTestsPassed':len(unique),'testCountIncludesRepeatedFilters':True,
 'uniqueTestIndexSha256':sha(OUT/'unique-passed-tests.json'),
 'limitsIncreased':False,'wasmPublished':False,'uiOrFocusInteraction':False,
 'sourceGuardVerified':True,'ownedMainMatches':True,'ownedSources':r['ownedSources'],
 'qualificationReceipt':'qualification-r9.json','qualificationReceiptSha256':sha(OUT/'qualification-r9.json'),
 'sourceIndex':'source-r9.json','sourceIndexSha256':r['sourceIndexSha256'],
 'sourceArchive':'qualified-source-r9.tar.gz','sourceArchiveSha256':r['sourceArchiveSha256'],
 'nativeTestBinariesSha256':binaries,
 'implemented':[
  'Exact whole-span Cartesian/rational polynomial compositions under strictly monotone covering maps',
  'Ordered local graph/projective charts including periodic one-lap maps and rejected double covers',
  'Exact projective linear inverse proposals for nondegenerate conics and full-rank spatial cubics',
  'Existing zeroth-jet facts route map proposals without additional arithmetic; exact normal-scale proofs retain priority',
  'Private immutable joint continuous/global/G0-G1-G2 tokens for four constructor-owned sweep body families',
  'Native JSON joint flags recomputed from owned tokens; caller flags cannot replace any stage'],
 'validated':[
  'Prior periodic-profile and curvature-jump shared-work regressions pass under unchanged budgets',
  'Hidden interior/one-ULP mutations and zero/one-short work cannot acquire certificates',
  'G1-only geometry cannot acquire G2; sharp filled-cap joins retain C0',
  'Source and retained global/G2 flags survive native STEP restoration for authored-affine and Bishop arc cases',
  'Sweep laws, RMF/holonomy, caps/decomposition, embedding, volume/orientation, nesting and intersection regressions'],
 'notClaimed':[
  'Universal nonlinear inverse coverage outside the verified Cartesian/projective chart families',
  'Unproved source/global sweep or miter combinations; exhausted or unsupported proofs remain unproved',
  'A new independent OpenCascade/207-case STEP matrix, UI matrix, or WASM publication'],
 'independent207MatrixReplayed':False,'nativeStepRoundTrips':True,
 'noCodecScope':'cad-predicates and nurbs-core only',
 'readme':'README.md','readmeSha256':sha(OUT/'README.md'),
 'failedInputDeltas':'failed-input-deltas.json','failedInputDeltasIndexSha256':sha(OUT/'failed-input-deltas.json'),
 'failedInputDeltasArchiveSha256':sha(OUT/'failed-input-deltas.tar.gz'),
 'driverSha256':sha(OUT/'qualify-r9.py'),'finalizerSha256':sha(__file__)
}
(OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:result[k]for k in ['state','testsPassed','uniqueTestsPassed','fullGoalComplete']}))
