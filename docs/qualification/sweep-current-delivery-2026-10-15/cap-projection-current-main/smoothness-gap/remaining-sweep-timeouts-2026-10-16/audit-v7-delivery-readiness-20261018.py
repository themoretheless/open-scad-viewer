"""Read-only release preflight; finite gates cannot complete the full objective."""
from pathlib import Path
import hashlib,json
root=Path.cwd();out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-boolean-repair-v7-source-20261016');base=Path('/private/tmp/osv-sweep-frozen-current-v5-source-20261016')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
expected='754411a7753ba91a9ea1a518d1aabfdfdc88f68260c1dd0ec031899e25c8b179'
checks=[]
def check(name,ok,detail):checks.append({'name':name,'passed':bool(ok),'detail':detail})
def receipt(name,phases=()):
 p=out/name
 if not p.exists():check(name,False,'missing');return None
 j=json.loads(p.read_text());actual={p['name']:p.get('state') for p in j.get('phases',[])}
 ok=j.get('state')=='complete' and j.get('passed') is True and all(actual.get(n)=='passed' for n in phases)
 check(name,ok,{'state':j.get('state'),'sha256':sha(p),'requiredPhases':list(phases),'actualPhases':actual})
 return j
native=receipt('v7-native-repair-tests-retry3-receipt.json')
build=receipt('frozen-current-v7-build.json')
provider=receipt('v7-current-native-profile-20261018-receipt.json')
runtime=receipt('v7-runtime-normal-priority-20261018.json',['original31','affine','public100','observer-unit'])
finalname='frozen-v7-normal-priority-20261018-independent-step-ui-final'
final=receipt(finalname+'/receipt.json',['multispan-constructed','multispan-packed-jets','multispan-closed-jets','canonical207-ui478','legacy-cap-refusal-reconciliation','retained-interiors-producer','retained-interiors','c2-step4-export','c2-step4-reference','c2-step4-opencascade','c2-ui8'])
if build:check('build artifact identity',build.get('packedSha256')==expected and build.get('sourceChangedDuringBuild') is False,build.get('packedSha256'))
if provider:check('complete provider baseline',provider.get('passed') is True and provider.get('providerTests')==provider.get('providerPassed') and provider.get('providerFailed')==0 and len(provider.get('testFiles',[]))==19,{'total':provider.get('providerTests'),'passed':provider.get('providerPassed'),'failed':provider.get('providerFailed')})
if runtime:
 five=runtime.get('originalFive',{})
 check('original five unchanged 30s deadlines',runtime.get('deadlineClosureConfirmed') is True and len(five)==5 and all(v.get('status')=='passed' and 0<v.get('durationMs',float('inf'))<30000 for v in five.values()),five)
 check('current public100 corpus',runtime.get('public100Tests',0)>=736,runtime.get('public100Tests'))
nativepath=stage/'main-native-source-sha256.json';runtimepath=stage/'runtime-source-sha256.json';natives=json.loads(nativepath.read_text());sources=json.loads(runtimepath.read_text())
drift=[n for n,h in natives.items() if not (root/n).is_file() or sha(root/n)!=h]
check('current main native inputs equal candidate',not drift,{'count':len(natives),'changed':drift,'ledgerSha256':sha(nativepath)})
check('candidate native input integrity',all((stage/n).is_file() and sha(stage/n)==h for n,h in natives.items()),sha(nativepath))
check('candidate runtime input integrity',all((stage/n).is_file() and sha(stage/n)==h for n,h in sources.items()),sha(runtimepath))
for n in ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm']:check('candidate '+n,sha(stage/n)==expected,sha(stage/n))
planned=['src/core/geometryExecution.ts','src/core/currentRuntimeGeometryEvidence.ts','tests/engineManifest.test.ts','tests/mcpServer.test.ts','tests/manifoldPlanBackend.test.ts','tests/brepNativeProgramSemantics.test.ts','tests/runtimeGeometryActualBinding.test.ts']
for n in planned:
 current=root/n;previous=base/n
 check('preserved pending source '+n,(current.exists() and previous.exists() and sha(current)==sha(previous)) or (not current.exists() and not previous.exists()),{'current':sha(current) if current.exists() else None,'qualified':sha(stage/n)})
critical=[]
for n,h in sources.items():
 if n in planned or n.startswith(('src/generated/geometry-kernels/','src/generated/wasm-brotli/')):continue
 if n.startswith(('src/','scripts/','integrations/','tools/')) or n in ['package.json','package-lock.json','index.html','vite.config.ts','tsconfig.json','tsconfig.mcp.json','THIRD_PARTY_NOTICES.md']:
  if not (root/n).is_file() or sha(root/n)!=h:critical.append(n)
check('other main product inputs preserved',not critical,critical)
old='0aa68af8a0b41bfcb0a6f912d20ae07ec40c5cfff42b6dc4128603c488e5e28c'
check('previous main kernel not replaced by a peer',sha(root/'public/wasm/geometry-kernel.wasm')==sha(root/'src/generated/geometry-kernels/kernel_bg.wasm')==old,sha(root/'public/wasm/geometry-kernel.wasm'))
objective=json.loads((out/'full-goal-requirements-current.json').read_text());obligations=objective.get('requirements',objective.get('obligations',[]));required={'continuous-bound','global','retained-smoothness','laws','step','ui','delivery'}
check('complete objective inventory',len(obligations)==7 and {r['id'] for r in obligations}==required,[r['id'] for r in obligations])
full_verified=all(r.get('status')=='verified' and r.get('authoritativeReceipts') for r in obligations)
result={'schema':'sweep-v7-readonly-release-preflight/1','candidateWasmSha256':expected,'readyForFiniteArtifactDelivery':all(c['passed'] for c in checks),'requirementsMarkedVerified':bool(full_verified),'fullObjectiveVerified':False,'fullGoalComplete':False,'scope':'Source/binding/release preflight only. Successful finite gates do not replace requirement-by-requirement geometric completion audit. No workspace or artifact mutations.','checks':checks,'pendingObjectiveRequirements':[r['id'] for r in obligations if r.get('status')!='verified'],'pendingSourceFiles':planned,'providerQualificationRecord':'docs/qualification/runtime-provider-754411a7753b-2026-10-16.json','providerQualificationRecordPresent':(root/'docs/qualification/runtime-provider-754411a7753b-2026-10-16.json').exists()}
report=out/'v7-delivery-readiness-current-20261018.json';t=report.with_suffix('.writing');t.write_text(json.dumps(result,indent=2)+'\n');t.replace(report)
print(json.dumps({'readyForFiniteArtifactDelivery':result['readyForFiniteArtifactDelivery'],'fullGoalComplete':False,'incompleteChecks':[c['name'] for c in checks if not c['passed']],'pendingObjectiveRequirements':result['pendingObjectiveRequirements']},indent=2))
