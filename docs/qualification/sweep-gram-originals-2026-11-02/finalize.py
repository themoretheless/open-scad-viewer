from pathlib import Path
import hashlib,json,re,tarfile
Q=Path(__file__).resolve().parent;ROOT=Q.parents[2];STAGE=Path('/private/tmp/osv-sweep-gram-20261102-r4')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
r=json.loads((Q/'audit-r4.json').read_text());assert r['auditComplete'] and r['state']=='complete' and not r['passed'] and r['testsFailed']==5
ledger=json.loads((Q/'source-r4.json').read_text());changed=json.loads((Q/'changed-r4.json').read_text())
assert all(sha(STAGE/n)==h for n,h in ledger.items());assert all(sha(ROOT/n)==h for n,h in changed.items())
unique=[]
failed=[]
for phase in r['phases']:
 log=Q/('r4-'+phase['name']+'.log');assert sha(log)==phase['logSha256'];assert phase['returncode']==(101 if phase['name']=='brep-sweep' else 0)
 names=re.findall(r'^test (.+) \.\.\. ok$',log.read_text(),re.M)
 assert len(names)==phase['testsPassed']
 unique.extend(phase['name']+'/'+n for n in names)
 failed.extend(phase['name']+'/'+n for n in re.findall(r'^test (.+) \.\.\. FAILED$',log.read_text(),re.M))
assert len(unique)==len(set(unique))==r['testsPassed']
assert len(failed)==len(set(failed))==r['testsFailed']
r.update(failedTests=failed,testsTotal=len(unique)+len(failed),boundedProofChecksPassed=True)
(Q/'unique-tests-r4.json').write_text(json.dumps(unique,indent=2)+'\n')
external=[]
for directory,filename in [('native-body','quintic-body.step'),('native-moving','moving-carrier.step')]:
 data=json.loads((Q/directory/'independent-occt.json').read_text());assert data['passed'];assert data['stepSha256']==sha(Q/directory/filename);external.append(data)
for name in ['construct','audit']:
 v=json.loads((Q/'native-moving'/(name+'.json')).read_text())
 for k in ['certified','continuousBound','jointContinuousGlobalCertified','jointContinuousGlobalG1Certified','jointContinuousGlobalG2Certified','fullBoundaryG1Certified','fullBoundaryG2Certified']:assert v[k] is True
for name in ['closure-gap','empty-volume-budget','interior-fold']:
 v=json.loads((Q/'native-moving'/('refused-'+name+'.json')).read_text());assert v['certified'] is False and v['jointContinuousGlobalG2Certified'] is False and v['model'] is None
for name in ['boundary','boundary-imported']:
 v=json.loads((Q/'native-body'/(name+'.json')).read_text());assert v['continuousBound'] is True and v['errorUpper']==0 and v['referenceRegularity']['proven'] is True and v['referenceVolume']['exactBoundary'] is True and v['certified'] is False and v['reason']=='solid-geometry-unproved'
trim=json.loads((Q/'native-body/trim-diagnostic.json').read_text());assert not trim['allValid'];assert [f['face']for f in trim['faces']if f['valid'] is None]==[2,3]
assert sha(Q/'audited-source-r4.tar.gz')==r['sourceArchiveSha256']
r.update(uniqueTestsPassed=len(unique),nativeProtocolTestsPassed=next(p['testsPassed']for p in r['phases']if p['name']=='native'),sourceIndexSha256=sha(Q/'source-r4.json'),changedIndexSha256=sha(Q/'changed-r4.json'),externalStepCasesPassed=len(external),externalOpenCascade=True,quinticOriginalBodyGlobalCertified=False,quinticImportedBodyGlobalCertified=False,originalToImportedUvAtlasCertified=False,completeMathematicalSweepCoverage=False,reportSha256=sha(Q/'README.md'))
# Keep ignored JSON/log/STEP evidence in a standalone durable archive. Source
# archives remain separate to avoid recursive archive/result checksums.
paths=sorted(p for p in Q.rglob('*')if p.is_file() and not p.name.endswith('.tar.gz') and p not in [Q/'result.json',Q/'receipt.md'])
with tarfile.open(Q/'qualification-evidence.tar.gz','w:gz',compresslevel=3)as t:
 for p in paths:t.add(p,arcname=str(p.relative_to(Q)),recursive=False)
r['evidenceArchiveSha256']=sha(Q/'qualification-evidence.tar.gz');(Q/'result.json').write_text(json.dumps(r,indent=2)+'\n')
(Q/'receipt.md').write_text('# Final receipt\n\n'+f"Audit NOT qualified: {r['testsPassed']} passed, {r['testsFailed']} failed. Passing tests ({r['uniqueTestsPassed']} unique), {r['nativeProtocolTestsPassed']} native protocol tests; no-codec check; two independent STEP cases.\n\n"+f"Source index SHA-256: `{r['sourceIndexSha256']}`.\n\nSource archive SHA-256: `{r['sourceArchiveSha256']}`.\n\nEvidence archive SHA-256: `{r['evidenceArchiveSha256']}`.\n\n"+'Limits unchanged; test optimization 1, assertions and overflow checks enabled. No focus/UI action, WASM publication or full-math completion. The new quintic body remains globally unproved at cap curve injectivity; original/STEP UV atlas correspondence remains unproved.\n')
print(json.dumps({k:r[k]for k in ['passed','auditComplete','testsPassed','testsFailed','testsTotal','uniqueTestsPassed','nativeProtocolTestsPassed','externalStepCasesPassed','fullGoalComplete']},indent=2))
