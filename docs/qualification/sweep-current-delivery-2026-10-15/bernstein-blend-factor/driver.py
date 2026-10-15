from pathlib import Path
import hashlib,json,os,subprocess,time,shutil
root=Path('/private/tmp/osv-sweep-bernstein-blend-candidate-20261015')
frozen=Path('/private/tmp/osv-sweep-interval-mul-wasm-20261015')
out=Path('/private/tmp/osv-sweep-bernstein-blend-qualification-20261015');out.mkdir(exist_ok=False)
source='crates/nurbs-core/src/distance/surface_distance/bezier_restriction.rs'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
ledger=json.loads((frozen/'frozen-runtime-source-sha256.json').read_text());native={p:h for p,h in ledger.items() if p.startswith('crates/')}
receipt={'state':'running','passed':False,'scope':'Isolated frozen N source; exact original hollow request, budgets and full proof/work equality; ABBA native performance. Not browser/full-goal qualification.','sourceLedgerSha256':sha(frozen/'frozen-runtime-source-sha256.json'),'phases':[]}
def save():
 p=out/'receipt.json.writing';p.write_text(json.dumps(receipt,indent=2)+'\n');p.replace(out/'receipt.json')
def check(candidate=False):
 for p,h in native.items():
  if candidate and p==source:continue
  assert sha(root/p)==h,p
 return len(native)
def run(name,argv):
 p={'name':name,'argv':list(map(str,argv)),'state':'running'};receipt['phases'].append(p);save();print(name+': started',flush=True);t=time.monotonic()
 with (out/(name+'.log')).open('w') as log:r=subprocess.run(p['argv'],cwd=root,env={**os.environ,'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2'},stdout=log,stderr=subprocess.STDOUT)
 p.update(state='passed' if r.returncode==0 else 'failed',returncode=r.returncode,seconds=time.monotonic()-t);save();print(name+': '+p['state'],flush=True)
 if r.returncode:receipt['state']='failed';save();raise SystemExit(r.returncode)
receipt['nativeFilesChecked']=check();save()
build=['cargo','build','--offline','--locked','--release','--manifest-path','crates/geometry-bridge/Cargo.toml','--example','bench_sweep_volume_request']
run('baseline-build',build);check()
binpath=root/'crates/target/release/examples/bench_sweep_volume_request'
baseline=out/'baseline-bin';shutil.copy2(binpath,baseline);receipt['baselineBinarySha256']=sha(baseline)
p=root/source;before=p.read_text();(out/'before.rs').write_text(before)
assert before.count('a[k].mul(beta)?.add(b[k].mul(alpha)?)?')==1
p.write_text(before.replace('a[k].mul(beta)?.add(b[k].mul(alpha)?)?','super::mul_blend_factor(a[k],beta)?.add(super::mul_blend_factor(b[k],alpha)?)?'))
receipt['candidateSourceSha256']=sha(p);(out/'candidate.rs').write_bytes(p.read_bytes());save();check(True)
for name,package,pattern in [('factor-bit-parity','nurbs-core','blend_factor_regression'),('bernstein-restriction','nurbs-core','bezier_restriction::tests'),('contact-traversal','brep-core','face_contacts::tests::resumable_contacts_match_frozen_traversal_and_cumulative_budgets')]:
 run(name,['cargo','test','--offline','--locked','--release','--manifest-path','crates/Cargo.toml','-p',package,pattern,'--lib','--','--test-threads=2']);check(True)
run('candidate-build',build);check(True)
candidate=out/'candidate-bin';shutil.copy2(binpath,candidate);receipt['candidateBinarySha256']=sha(candidate);save()
request=Path('/private/tmp/osv-sweep-current-closed-hollow-capture-20261015.jsonl.volume-request.json');reference_path=Path('/private/tmp/osv-sweep-current-closed-hollow-native-volume-20261015.json.report.json');reference=json.loads(reference_path.read_text())
receipt['requestSha256']=sha(request);receipt['referenceReportSha256']=sha(reference_path);receipt['measurements']=[];save()
for i,kind in enumerate(['baseline','candidate','candidate','baseline']):
 check(True);target=out/(str(i)+'-'+kind+'.json');run(str(i)+'-'+kind,[baseline if kind=='baseline' else candidate,request,target])
 proof=Path(str(target)+'.report.json');assert json.loads(proof.read_text())==reference,'Complete native proof/work changed'
 d=json.loads(target.read_text());receipt['measurements'].append({'kind':kind,'milliseconds':d['milliseconds'],'steps':d['steps'],'completeProofMatchesReference':True,'reportSha256':sha(proof)});save()
means={k:sum(x['milliseconds'] for x in receipt['measurements'] if x['kind']==k)/2 for k in ['baseline','candidate']}
receipt.update(state='complete',passed=True,meanMilliseconds=means,improvementPercent=100*(1-means['candidate']/means['baseline']),retainedInMain=False)
check(True);save();print(json.dumps({'state':'complete','meanMilliseconds':means,'improvementPercent':receipt['improvementPercent']}),flush=True)
