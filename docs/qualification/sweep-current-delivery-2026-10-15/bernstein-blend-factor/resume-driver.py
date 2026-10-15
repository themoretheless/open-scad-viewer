from pathlib import Path
import hashlib,json,os,subprocess,time,shutil
root=Path('/private/tmp/osv-sweep-bernstein-blend-candidate-20261015')
frozen=Path('/private/tmp/osv-sweep-interval-mul-wasm-20261015')
prior_root=Path('/private/tmp/osv-sweep-bernstein-blend-qualification-20261015')
out=Path('/private/tmp/osv-sweep-bernstein-blend-resume-20261015');out.mkdir(exist_ok=False)
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
prior=json.loads((prior_root/'receipt.json').read_text());assert prior['state']=='failed' and prior['passed'] is False
assert [(p['name'],p['state']) for p in prior['phases']]==[('baseline-build','passed'),('factor-bit-parity','passed'),('bernstein-restriction','passed'),('contact-traversal','failed')]
assert prior['phases'][-1]['returncode']==101
assert "couldn't read" in (prior_root/'contact-traversal.log').read_text()
fixture_path=prior_root/'test-fixture-repair.json';fixtures=json.loads(fixture_path.read_text());assert fixtures['unresolved']==[] and len(fixtures['files'])==8
source='crates/nurbs-core/src/distance/surface_distance/bezier_restriction.rs'
ledger=json.loads((frozen/'frozen-runtime-source-sha256.json').read_text());native={p:h for p,h in ledger.items() if p.startswith('crates/')}
baseline=prior_root/'baseline-bin';assert sha(baseline)==prior['baselineBinarySha256']
expected=(prior_root/'before.rs').read_text().replace('a[k].mul(beta)?.add(b[k].mul(alpha)?)?','super::mul_blend_factor(a[k],beta)?.add(super::mul_blend_factor(b[k],alpha)?)?')
assert hashlib.sha256(expected.encode()).hexdigest()==prior['candidateSourceSha256']
receipt={'state':'running','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'priorReceiptSha256':sha(prior_root/'receipt.json'),'fixtureRepairSha256':sha(fixture_path),'nativeSourceLedgerSha256':sha(frozen/'frozen-runtime-source-sha256.json'),'nativeFilesChecked':len(native),'baselineBinarySha256':sha(baseline),'candidateSourceSha256':prior['candidateSourceSha256'],'retainedPassedPhases':[p for p in prior['phases'] if p['state']=='passed'],'phases':[],'scope':'Resume only failed contact compilation after restoring8test-only external files, then native candidate build/full-proof/work-equal ABBA. No prior export/test phases rerun or failure relabelled. No WASM/UI/main delivery claim.','retainedInMain':False}
def save():
 p=out/'receipt.json.writing';p.write_text(json.dumps(receipt,indent=2)+'\n');p.replace(out/'receipt.json')
def identity():
 assert sha(__file__)==receipt['driverSha256']
 assert sha(baseline)==receipt['baselineBinarySha256']
 for p,h in native.items():
  if p==source:assert (root/p).read_text()==expected
  else:assert sha(root/p)==h,p
 for f in fixtures['files']:assert sha(root/f['file'])==f['sha256'],f['file']
def run(name,argv):
 identity();p={'name':name,'argv':list(map(str,argv)),'state':'running'};receipt['phases'].append(p);save();print(name+': started',flush=True);t=time.monotonic()
 with (out/(name+'.log')).open('w') as log:r=subprocess.run(p['argv'],cwd=root,env={**os.environ,'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2'},stdout=log,stderr=subprocess.STDOUT)
 identity();p.update(state='passed' if r.returncode==0 else 'failed',returncode=r.returncode,seconds=time.monotonic()-t);save();print(name+': '+p['state'],flush=True)
 if r.returncode:receipt['state']='failed';save();raise SystemExit(r.returncode)
identity();save();print('resume pid',os.getpid(),flush=True)
run('contact-traversal',['cargo','test','--offline','--locked','--release','--manifest-path','crates/Cargo.toml','-p','brep-core','face_contacts::tests::resumable_contacts_match_frozen_traversal_and_cumulative_budgets','--lib','--','--test-threads=2'])
run('candidate-build',['cargo','build','--offline','--locked','--release','--manifest-path','crates/geometry-bridge/Cargo.toml','--example','bench_sweep_volume_request'])
candidate=out/'candidate-bin';shutil.copy2(root/'crates/target/release/examples/bench_sweep_volume_request',candidate);receipt['candidateBinarySha256']=sha(candidate);save()
request=Path('/private/tmp/osv-sweep-current-closed-hollow-capture-20261015.jsonl.volume-request.json');reference_path=Path('/private/tmp/osv-sweep-current-closed-hollow-native-volume-20261015.json.report.json');reference=json.loads(reference_path.read_text());receipt.update(requestSha256=sha(request),referenceReportSha256=sha(reference_path),measurements=[]);save()
for i,kind in enumerate(['baseline','candidate','candidate','baseline']):
 target=out/(str(i)+'-'+kind+'.json');run(str(i)+'-'+kind,[baseline if kind=='baseline' else candidate,request,target]);proof=Path(str(target)+'.report.json');assert json.loads(proof.read_text())==reference,'Entire native proof/work changed'
 d=json.loads(target.read_text());receipt['measurements'].append({'kind':kind,'milliseconds':d['milliseconds'],'steps':d['steps'],'completeProofMatchesReference':True,'reportSha256':sha(proof)});save()
means={k:sum(x['milliseconds'] for x in receipt['measurements'] if x['kind']==k)/2 for k in ['baseline','candidate']}
receipt.update(state='complete',passed=True,meanMilliseconds=means,improvementPercent=100*(1-means['candidate']/means['baseline']));identity();save();print(json.dumps({'state':'complete','meanMilliseconds':means,'improvementPercent':receipt['improvementPercent']}),flush=True)
