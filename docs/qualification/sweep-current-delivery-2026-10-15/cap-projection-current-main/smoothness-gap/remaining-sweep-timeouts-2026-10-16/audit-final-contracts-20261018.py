"""Bind the reviewed conditional native contracts to the delivered qualification.

This gate checks the evidence for the source audit in final-contract-audit.md;
fixture counts by themselves cannot establish its mode-independent invariants.
"""
from pathlib import Path
import hashlib,importlib.util,json,datetime,re
OUT=Path(__file__).resolve().parent;ROOT=OUT.parents[5];STAGE=Path('/private/tmp/osv-sweep-common-source-v11-20261018')
EXPECTED='68dd0ed6d368a7f4e5beb8e1a65c26720b8e6316c54a4f953e1ee49161ca4bd1'
BASE='frozen-v11r1-common-source-20261018-independent-step-ui-final'
inputs={}
def sha(path):
 with Path(path).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()
def read(name):
 p=OUT/name;inputs[str(p)]=sha(p);return json.loads(p.read_text())
def complete(name):
 d=read(name);assert d.get('state')=='complete' and d.get('passed')is True,name;return d

def main():
 result_path=OUT/'v11-final-contract-audit-20261018.json';assert not result_path.exists(),'Preserve prior completion audits'
 ledger=read('full-goal-requirements-current.json');assert {r['id']for r in ledger['requirements']}=={'continuous-bound','global','retained-smoothness','laws','step','ui','delivery'}and len(ledger['requirements'])==7
 runtime=complete('v11r1-common-source-runtime-20261018.json');assert runtime['packedSha256']==EXPECTED
 assert runtime['deadlineClosureConfirmed'] and runtime['public100Tests']>=736 and runtime['providerBaselineTests']>=303 and runtime['commonPeerHostTests']>=153
 assert len(runtime['originalFive'])==5 and all(x['status']=='passed'and 0<x['durationMs']<30000 for x in runtime['originalFive'].values())
 primary=complete('v11-primary-delivery-20261018.json');verification=complete('v11-primary-runtime-verification-20261018.json');assert primary['geometryWasmSha256']==verification['geometryWasmSha256']==EXPECTED
 assert sha(verification_path:=Path(primary['primaryRuntimeVerification']))==primary['primaryRuntimeVerificationSha256']
 preflight=read('v11-delivery-readiness-current-20261018.json');assert preflight['readyForFiniteArtifactDelivery'] and all(c['passed']for c in preflight['checks'])
 producer=read('v11-final-runtime-provider-execution-20261018.json');assert producer['matches'] and producer['publicArtifactMatches'] and producer['artifactSha256']==EXPECTED and producer['compilerLoads']==1
 assert {r['engineClass']for r in producer['records']}=={'mesh','brep'}and all(r['matches']and r['runtimeEvidence']and r['volumeMm3']>0 for r in producer['records'])
 plan=read('v11-delivery-plan-20261018.json');spec=importlib.util.spec_from_file_location('delivery_inventory',OUT/'prepare-v11-delivery-20261018.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
 assert len(plan['targets'])==12 and all(module.inventory(ROOT/e['path'])==e['after']for e in plan['targets'])
 assert all(module.inventory(Path(primary['backup'])/'before'/e['path'])==e['before']for e in plan['targets']if e['before']is not None)
 native_ledger=json.loads((STAGE/'main-native-source-sha256.json').read_text());source_ledger=json.loads((STAGE/'runtime-source-sha256.json').read_text())
 compiled={n:h for n,h in source_ledger.items()if n.startswith(('src/','crates/'))or n in ['package.json','package-lock.json','index.html','vite.config.ts','tsconfig.json','tsconfig.mcp.json']}
 assert all(sha(ROOT/n)==h==sha(STAGE/n)for n,h in compiled.items())
 for n in ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm']:assert sha(ROOT/n)==EXPECTED
 review=read('v11-current-proof-scope-review-20261018.json');assert review['artifactSha256']==EXPECTED
 for e in review['reviewedCurrentEntryPoints']:assert sha(ROOT/e['source'])==e['sha256']==sha(STAGE/e['source'])
 for e in review['previousCompositionReviewApplicability']:assert e['unchangedOwnerSource'] and sha(ROOT/e['source'])==e['currentSha256']==sha(STAGE/e['source'])
 additional_sources=['crates/brep-core/src/sweep_smoothness.rs','crates/brep-core/src/sweep_smoothness/geometric.rs','crates/brep-core/src/sweep_retained_charts.rs','crates/brep-core/src/face_injectivity.rs','crates/brep-core/src/sweep_miter_owned/annular.rs','crates/nurbs-core/src/join/continuity/exact_strip.rs','crates/nurbs-core/src/join/continuity/projective_collection.rs','crates/nurbs-core/src/analysis/surface_injectivity.rs','crates/nurbs-core/src/analysis/surface_linear_monotonicity.rs','src/services/solidBridge.ts','crates/geometry-bridge/src/cad_geometric_g2.rs']
 source_review={e['source']:e['sha256']for e in review['reviewedCurrentEntryPoints']}
 for n in additional_sources:assert sha(ROOT/n)==sha(STAGE/n)==compiled[n];source_review[n]=sha(ROOT/n)
 orientation=(ROOT/'crates/nurbs-core/src/sweeps/progressive_sweep.rs').read_text().split('pub enum Orientation {',1)[1].split('}',1)[0]
 assert set(re.findall(r'^    (\w+),',orientation,re.M))=={'RotationMinimizing','Fixed','FixedNormal','Frenet','CorrectedFrenet'}
 parent=complete(BASE+'/receipt.json');canonical=complete(BASE+'/canonical207-ui478/receipt.json');assert parent['geometryWasmSha256']==canonical['geometryWasmSha256']==EXPECTED
 global_report=read(BASE+'/canonical207-ui478/step207/global.json');bounds=read(BASE+'/canonical207-ui478/step207/complete-boundary.json');finite=read(BASE+'/canonical207-ui478/step207/finite-modes.json')
 assert global_report['passed']and global_report['certified']==205 and global_report['refused']==2
 assert bounds['passed']and bounds['certified']==206 and bounds['unproved']==1 and not bounds['unclassified']
 assert len(finite['smoothness'])==207 and len(finite['lawCases'])==16 and len(finite['spatial'])==22
 ui=read(BASE+'/canonical207-ui478/ui478/full-reconciliation.json');assert ui['passed']and ui['cases']==478 and ui['nativeObservations']==1472 and ui['geometryWasmSha256']==EXPECTED
 for group in ui['groups']:assert sha(group['reportPath'])==group['sha256']
 for name,cases,obs in [('v11-native-rush-ui8-followup-20261018.json',8,32),('v11-surface-native-followup-20261018.json',110,220)]:
  d=complete(name);assert d['geometryWasmSha256']==EXPECTED and d['cases']==cases and d['nativeObservations']==obs
  assert all(sha(n)==h for n,h in d['inputsSha256'].items())
 complete('v11-companion27-followup-20261018.json');contracts=read('v11-companion27-current-contracts-20261018.json');assert contracts['passed']and len(contracts['cases'])==27 and contracts['geometryWasmSha256']==EXPECTED
 interiors=read(BASE+'/retained-interiors/result.json');assert interiors['passed']and interiors['summary']=={'cases':207,'surfaces':36876,'unprovedBasisC1':[],'unprovedBasisC2':[]}
 inventory=read('v11-declared-mode-coverage-20261018.json');assert inventory['allDispatchOwnersHavePositiveWitness']and not inventory['missingPositiveOwners'] and len(inventory['rows'])==239
 assert {x['owner']for x in inventory['summary']}=={'rmf','fixed','fixed-normal','frenet','corrected-frenet','authored','guided','contact','miter'}
 source_step=complete('v11-original-source-step37-occt-20261018.json');assert source_step['cases']==37 and source_step['geometryWasmSha256']==EXPECTED and sha(source_step['manifest'])==source_step['manifestSha256']and sha(source_step['report'])==source_step['reportSha256']
 new_manifest=json.loads(Path(source_step['manifest']).read_text());positive=[r for r in inventory['rows']if r['positiveSolid']];new_sources={c['sourceSha256']for c in new_manifest['cases']}
 assert len(positive)==183 and all(r['nativeStepWitnesses']or r['sourceSha256']in new_sources for r in positive)
 extra_basis=read('v11-original-source-interiors37-20261018/result.json');seams=read('v11-retained-proof-gaps9-20261018.json');composition=read('v11-source-proof-composition37-final-20261018.json')
 assert extra_basis['passed']and extra_basis['surfaces']==2386 and len(extra_basis['cases'])==37
 assert seams['passed']and len(seams['cases'])==9
 assert composition['passed']and len(composition['cases'])==37 and len(composition['controls'])==6 and all(c['refused']for c in composition['controls'])
 assert composition['scriptSha256']==sha(ROOT/'scripts/verify-sweep-source-composition.mts')and all(sha(n)==h for n,h in composition['inputsSha256'].items())
 assert all(c['geometricInteriorPrerequisitesPresent']for c in composition['cases'])
 assert composition['independentCaps'][0]['exactWork']<=composition['independentCaps'][0]['maxWork']==2000000
 for d in [interiors,extra_basis,seams,composition]:assert d['geometryWasmSha256']==EXPECTED
 for n in ['continuous-bound-composition-source-review.json','native-timeout-optimization-proof-review.json','final-contract-audit-20261018.md']:
  inputs[str(OUT/n)]=sha(OUT/n)
 requirements=[{'id':r['id'],'requirement':r['requirement'],'status':'verified',
   'basis':{'continuous-bound':'Exhaustive current native dispatch/composition ownership; full-domain conditional source/correction/decomposition/cap admission; actual canonical/new source reports and refusal controls.',
            'global':'One actual-model native boundary/rank/all-pairs/nesting/orientation admission across modes; actual transferred/replayed model binding; all current positive and intended-negative cases.',
            'retained-smoothness':'Native actual retained strip/regularity/owned-edge scope; basis plus actual rank premises; all applicable declared retained contracts and explicit C0/refusal cases.',
            'laws':'All16native open/closed affine/frame/guide combinations, corrected companions and both-width exact-source actual Solid/UI.',
            'step':'Independent canonical207, annular/C2 supplements and original-source37 with original157preservation and negative controls.',
            'ui':'Full478plusC2/originalRush/Surface-native supplements, actual CPU-authenticated cancellation/source replacement and restoration at both widths.',
            'delivery':'Exact common-source Rust/WASM/runtime/application bytes delivered locally with original backups, unchanged original budgets and current actual native provider execution.'}[r['id']]}for r in ledger['requirements']]
 # Status/progress fields are updated after this audit. Bind the original
 # requirement definitions without creating a circular ledger/result hash.
 inputs.pop(str(OUT/'full-goal-requirements-current.json'))
 definitions=[{'id':r['id'],'requirement':r['requirement']}for r in ledger['requirements']]
 definition_hash=hashlib.sha256(json.dumps(definitions,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()).hexdigest()
 result={'schema':'sweep-final-original-contract-audit/1','passed':True,'allOriginalRequirementsVerified':True,'fullGoalComplete':True,
  'geometryWasmSha256':EXPECTED,'reviewedAtUtc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
  'scope':'Completion of the original library contracts: conditional whole-domain native admission across every declared mode, exhaustive source/control-flow ownership review, and full current finite STEP/UI/runtime delivery. Does not promise certification of arbitrary invalid/work-exhausted inputs or infer an all-input theorem from examples.',
  'requirements':requirements,'requirementsDefinitionSha256':definition_hash,'pendingRequirements':[],'sourceReviewSha256':source_review,'compiledInputsVerified':len(compiled),
  'inputsSha256':inputs,'auditScriptSha256':sha(__file__),'auditDocument':str(OUT/'final-contract-audit-20261018.md'),
  'negativeCasesRemainRefused':['open-spatial-hollow-miter.step','affine-oblique-hollow-miter.step','folded Fixed closed body'],
  'localDeliveryOnly':True,'gitPushOrCleanupClaimed':False}
 result_path.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
 print('Final original-contract audit passed:7/7; no pending library requirement. Conditional native admission, source ownership and complete current qualification/delivery reconciled.')

if __name__=='__main__':main()
