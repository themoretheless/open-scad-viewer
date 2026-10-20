"""Read-only current common-source preflight. Does not certify the full objective."""
from pathlib import Path
import hashlib,json
root=Path.cwd();out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-common-source-v11-20261018')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
checks=[]
def check(name,ok,detail):checks.append({'name':name,'passed':bool(ok),'detail':detail})
def read(path):
 if not path.is_file():return None
 try:return json.loads(path.read_text())
 except json.JSONDecodeError:return None

def receipt(name,phases=()):
 p=out/name;j=read(p)
 if j is None:check(name,False,'missing or being written');return None
 states={x['name']:x.get('state') for x in j.get('phases',[])}
 check(name,j.get('state')=='complete' and j.get('passed') is True and all(states.get(n)=='passed' for n in phases),{'state':j.get('state'),'sha256':sha(p),'requiredPhases':list(phases),'actualPhases':states})
 return j
capture=read(out/'v11-common-source-capture-20261018.json');assert capture
runtime=receipt('v11r1-common-source-runtime-20261018.json',['common-profile-certificate','common-minkowski3d','common-bezier-profile','native-brep-regression','geometry-build','native-migration-producer','vue-typecheck','mcp-typecheck','application','distribution','runtime-identity','original31','affine','public100','provider-baseline','common-peer-host','common-peer-ui','observer-unit'])
build=receipt('frozen-current-v11-build.json')
finalname='frozen-v11r1-common-source-20261018-independent-step-ui-final'
final=receipt(finalname+'/receipt.json',['multispan-constructed','multispan-packed-jets','multispan-closed-jets','canonical207-ui478','legacy-cap-refusal-reconciliation','retained-interiors-producer','retained-interiors','c2-step4-export','c2-step4-reference','c2-step4-opencascade','c2-ui8'])
expected=build.get('packedSha256') if build else None
check('current build artifact identity',bool(expected) and build.get('sourceChangedDuringBuild') is False and runtime and runtime.get('packedSha256')==expected,expected)
if runtime:
 five=runtime.get('originalFive',{})
 check('five original unchanged deadlines',runtime.get('deadlineClosureConfirmed') is True and len(five)==5 and all(v.get('status')=='passed' and 0<v.get('durationMs',float('inf'))<30000 for v in five.values()),five)
 check('complete public100',runtime.get('public100Tests',0)>=736,runtime.get('public100Tests'))
 check('complete provider baseline',runtime.get('providerBaselineTests',0)>=303 and runtime.get('providerBaselineFiles')==19,{'tests':runtime.get('providerBaselineTests'),'files':runtime.get('providerBaselineFiles')})
 check('common-source host and UI preservation',runtime.get('commonPeerHostTests',0)>=153 and runtime.get('commonPeerUiTests')==5,{'host':runtime.get('commonPeerHostTests'),'ui':runtime.get('commonPeerUiTests')})
 if build:check('runtime/build receipt binding',build.get('runtimeReceipt')==str(out/'v11r1-common-source-runtime-20261018.json'),build.get('runtimeReceipt'))
nativepath=stage/'main-native-source-sha256.json';runtimepath=stage/'runtime-source-sha256.json';natives=read(nativepath);sources=read(runtimepath)
check('candidate native input integrity',all((stage/n).is_file() and sha(stage/n)==h for n,h in natives.items()) and sha(nativepath)==capture['nativeLedgerSha256'],sha(nativepath))
check('candidate runtime input integrity',all((stage/n).is_file() and sha(stage/n)==h for n,h in sources.items()) and runtime and sha(runtimepath)==runtime.get('sourceLedgerSha256'),sha(runtimepath))
planned=capture['explicitPrivateOverrides']
def current_hash(n):return sha(root/n) if (root/n).is_file() else None
before=all(current_hash(n)==h['mainSha256'] for n,h in planned.items());after=all(current_hash(n)==sha(stage/n) for n in planned)
check('planned source mutation before/after tuple is coherent',before or after,{'before':before,'after':after,'files':list(planned)})
drift=[n for n,h in natives.items() if n not in planned and current_hash(n)!=h]
check('other main native source preserved',not drift,drift)
critical=[]
for n,h in sources.items():
 if n in planned or n.startswith(('src/generated/geometry-kernels/','src/generated/wasm-brotli/')):continue
 if n.startswith(('src/','scripts/','integrations/','tools/')) or n in ['package.json','package-lock.json','index.html','vite.config.ts','tsconfig.json','tsconfig.mcp.json','THIRD_PARTY_NOTICES.md']:
  if current_hash(n)!=h:critical.append(n)
check('other main product inputs preserved',not critical,critical)
# Cargo discovers new build.rs files and workspace members; application/native
# globs can also consume newly added files without changing an old input.
# Preserve unrelated peer qualification scripts, but refuse an uncaptured
# production source set instead of comparing only the old ledger's entries.
production_roots=[root/'src',*sorted((root/'crates').glob('*/src'))]
production_files={p.relative_to(root).as_posix() for directory in production_roots for p in directory.rglob('*') if p.is_file() and '__pycache__' not in p.parts}
production_files.update(p.relative_to(root).as_posix() for pattern in ('*/Cargo.toml','*/build.rs') for p in (root/'crates').glob(pattern) if p.is_file())
new_production=sorted(n for n in production_files if n not in sources and not n.startswith(('src/generated/geometry-kernels/','src/generated/wasm-brotli/')))
check('no uncaptured primary production inputs',not new_production,new_production)
for n in ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm']:
 check('candidate '+n,bool(expected) and (stage/n).is_file() and sha(stage/n)==expected,sha(stage/n) if (stage/n).is_file() else None)
mainkernels=[current_hash(n) for n in ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm']]
check('main artifact tuple preserved or coherently published',mainkernels==[capture['mainPackedWasmSha256']]*3 or bool(expected) and mainkernels==[expected]*3,mainkernels)
canon=receipt(finalname+'/canonical207-ui478/receipt.json',['step207-export','polynomial-reference','rational-reference','opencascade207','complete-boundary','global','finite-smoothness','inventory207','pair-cover','smoothness-controls','fresh-migration','original157','new-station-step','new-station-reference','new-station-occt','full-ui478'])
if canon:check('canonical exact artifact identity',bool(expected) and canon.get('geometryWasmSha256')==expected,canon.get('geometryWasmSha256'))
uipath=out/finalname/'canonical207-ui478/ui478/full-reconciliation.json';ui=read(uipath)
check('complete exact-artifact headless UI matrix',ui and ui.get('passed') is True and ui.get('geometryWasmSha256')==expected and ui.get('cases')==478 and ui.get('nativeObservations')==1472,{'receipt':str(uipath),'cases':ui.get('cases') if ui else None,'nativeObservations':ui.get('nativeObservations') if ui else None})
followup=receipt('v11-companion27-followup-20261018.json')
contract_path=out/'v11-companion27-current-contracts-20261018.json';contracts=read(contract_path)
check('all27 current corrected companion source contracts',bool(expected) and followup and followup.get('state')=='complete' and followup.get('passed') is True and followup.get('geometryWasmSha256')==expected and contracts and contracts.get('passed') is True and contracts.get('geometryWasmSha256')==expected and len(contracts.get('cases',[]))==27 and followup.get('reportSha256')==sha(contract_path) and contracts.get('canonicalReceiptSha256')==sha(out/finalname/'canonical207-ui478/receipt.json') and contracts.get('checkerSha256')==sha(out/'check_companion27_contracts.py'),{'currentReceipt':str(contract_path),'cases':len(contracts.get('cases',[])) if contracts else None})
extra_ui=receipt('v11-native-rush-ui8-followup-20261018.json',[
 'rush-unclamped-hollow','rush-periodic-hollow','rush-periodic-frame-guide-affine-hollow','unsegmented-frame-guide-affine-hollow-miter'])
check('four original Rush STEP inputs at both UI widths',extra_ui and extra_ui.get('state')=='complete' and extra_ui.get('passed') is True and extra_ui.get('geometryWasmSha256')==expected and extra_ui.get('cases')==8 and extra_ui.get('nativeObservations')==32 and len(extra_ui.get('fixtures',[]))==4 and all(Path(n).is_file() and sha(Path(n))==h for n,h in extra_ui.get('inputsSha256',{}).items()) and all(Path(n).is_file() and sha(Path(n))==h for n,h in extra_ui.get('prerequisitesSha256',{}).items()) and len(extra_ui.get('prerequisitesSha256',{}))==2,{'receipt':'v11-native-rush-ui8-followup-20261018.json','cases':extra_ui.get('cases') if extra_ui else None,'nativeObservations':extra_ui.get('nativeObservations') if extra_ui else None})
surface=receipt('v11-surface-native-followup-20261018.json')
check('all110 Surface cases have real-native build cancellation',surface and surface.get('state')=='complete' and surface.get('passed') is True and surface.get('geometryWasmSha256')==expected and surface.get('cases')==110 and surface.get('nativeObservations')==220 and len(surface.get('observations',[]))==220 and surface.get('report') and Path(surface['report']).is_file() and sha(Path(surface['report']))==surface.get('reportSha256') and all(Path(n).is_file() and sha(Path(n))==h for n,h in surface.get('inputsSha256',{}).items()) and surface.get('prerequisiteReceipt') and Path(surface['prerequisiteReceipt']).is_file() and sha(Path(surface['prerequisiteReceipt']))==surface.get('prerequisiteSha256'),{'receipt':'v11-surface-native-followup-20261018.json','cases':surface.get('cases') if surface else None,'nativeObservations':surface.get('nativeObservations') if surface else None})
objective=read(out/'full-goal-requirements-current.json');obligations=objective['requirements'];required={'continuous-bound','global','retained-smoothness','laws','step','ui','delivery'}
check('full original objective inventory preserved',len(obligations)==7 and {r['id'] for r in obligations}==required,[r['id'] for r in obligations])
result={'schema':'sweep-v11-readonly-release-preflight/1','candidateWasmSha256':expected,'readyForFiniteArtifactDelivery':all(c['passed'] for c in checks),'fullObjectiveVerified':False,'fullGoalComplete':False,'scope':'Common-source/binding/finite release preflight only. A separate requirement-by-requirement source/proof audit must establish full objective completion. No product or workspace mutations.','checks':checks,'pendingObjectiveRequirements':[r['id'] for r in obligations if r.get('status')!='verified'],'pendingSourceFiles':list(planned)}
p=out/'v11-delivery-readiness-current-20261018.json';tmp=p.with_suffix('.writing');tmp.write_text(json.dumps(result,indent=2)+'\n');tmp.replace(p)
print(json.dumps({'readyForFiniteArtifactDelivery':result['readyForFiniteArtifactDelivery'],'fullGoalComplete':False,'incompleteChecks':[c['name'] for c in checks if not c['passed']]},indent=2))
