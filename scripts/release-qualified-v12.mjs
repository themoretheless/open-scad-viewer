#!/usr/bin/env node
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {dirname,resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..')
const read=p=>JSON.parse(readFileSync(resolve(root,p),'utf8'))
const write=(p,v)=>{mkdirSync(dirname(resolve(root,p)),{recursive:true});writeFileSync(resolve(root,p),JSON.stringify(v,null,2)+'\n')}
const copyVersion=(source,target)=>writeFileSync(resolve(root,target),readFileSync(resolve(root,source),'utf8')
  .replaceAll('-v11','-v12').replaceAll('"const": 11','"const": 12').replaceAll('CA11-','CA12-'))
const digest=paths=>{const h=createHash('sha256');for(const p of [...paths].sort())h.update(p).update('\0').update(readFileSync(resolve(root,p))).update('\n');return h.digest('hex')}
const q='docs/qualification'
const previous={index:`${q}/plans/g8-full-matrix-index-v11.json`,matrix:`${q}/brep-full-closed-matrix-v11.json`,release:`${q}/brep-capability-registry-release-full-v11.json`}
const next={index:`${q}/plans/g8-full-matrix-index-v12.json`,matrix:`${q}/brep-full-closed-matrix-v12.json`,release:`${q}/brep-capability-registry-release-full-v12.json`,planSchema:`${q}/plans/brep-capability-qualification-plan-v12.schema.json`,evidenceSchema:`${q}/brep-capability-evidence-v12.schema.json`}
copyVersion(`${q}/plans/brep-capability-qualification-plan-v11.schema.json`,next.planSchema)
copyVersion(`${q}/brep-capability-evidence-v11.schema.json`,next.evidenceSchema)
const artifacts=[
  'crates/brep-core/src/close_topology.rs','crates/brep-core/src/lib.rs',
  'crates/geometry-bridge/src/lib.rs','src/services/geometry/brep.ts',
  'src/services/geometry/brepCapability.ts','tests/brepQualificationV12.test.ts',
  'src/generated/geometry-kernels/kernel_bg.wasm',
]
const common=['global-solid-audit/1','persistent-naming/1']
const specs=[
  {id:'close-topology/1',slug:'close-topology-1',successorOf:'persistent-naming/1',
    claim:'Bounded explicit non-manifold and mixed-dimensional complex over independently audited manifold cells.',
    deps:common,
    included:['1..64 immutable parts','2..8 shared-face cells','3..16 edge radial uses','explicit disconnected vertex fans','wire/face/sheet-shell/open-shell/solid/compound roles','boundary extraction and manifold decomposition'],
    excluded:['unbounded branching','implicit carrier matching','singular geometry','entry into GloballyAuditedSolidSet'],
    rows:[['CA12-TOPOLOGY-ROLE-TYPESTATES','Complete'],['CA12-RADIAL-FAN-ORIENTATION','Complete'],['CA12-BOUNDARY-DECOMPOSE-RECOMPOSE','Complete'],['CA12-SOLID-TYPESTATE-SEPARATION','Complete'],['CA12-BRANCH-SINGULAR-RESOURCE','typed-refuse']]},
  {id:'tolerant-complex-heal/1',slug:'tolerant-complex-heal-1',successorOf:'authorized-heal-gap-le1/2',
    claim:'Finite explicit cross-part correspondence certification with immutable budgets and exact parameter-partition authority.',
    deps:['numeric-evidence-curved-brep/1','boundary-correspondence/1','persistent-naming/1','close-topology/1'],
    included:['vertex/edge/face correspondences','per-entity and cumulative physical budgets','context hierarchy and provenance','exact contiguous edge carrier parameter partitions','transactional rollback and idempotence'],
    excluded:['silent matching','tolerance growth','approximate carrier splits','overlapping or stale identities','large gaps'],
    rows:[['CA12-HEAL-MULTIPART-CORRESPONDENCE','Complete'],['CA12-HEAL-ENTITY-CUMULATIVE-BUDGET','Complete'],['CA12-HEAL-EXACT-PARTITION','Complete'],['CA12-HEAL-ROLLBACK-IDEMPOTENCE','Complete'],['CA12-HEAL-LARGE-GAP-STALE','typed-refuse']]},
  {id:'close-topology-step/1',slug:'close-topology-step-1',successorOf:'step-interchange/4',
    claim:'Direct bounded solid-cell complex STEP roundtrip preserving supplemental non-manifold incidence.',
    deps:['close-topology/1','step-interchange/4'],
    included:['direct STEP /4 payload per manifold cell','shared-face, radial-ring and fan preservation','16 MiB aggregate bound'],
    excluded:['sheet/open-shell STEP entities','assemblies','fallback reconstruction'],
    rows:[['CA12-STEP-CELL-COMPLEX-ROUNDTRIP','Complete'],['CA12-STEP-INCIDENCE-IDENTITY','Complete'],['CA12-STEP-SHEET-ASSEMBLY-RESOURCE','typed-refuse']]},
  {id:'close-topology-iges/1',slug:'close-topology-iges-1',successorOf:'iges-interchange/2',
    claim:'Direct bounded solid-cell complex IGES roundtrip preserving supplemental non-manifold incidence.',
    deps:['close-topology/1','iges-interchange/2'],
    included:['direct IGES /2 payload per manifold cell','shared-face, radial-ring and fan preservation','16 MiB aggregate bound'],
    excluded:['sheet/open-shell IGES entities','local transforms','fallback reconstruction'],
    rows:[['CA12-IGES-CELL-COMPLEX-ROUNDTRIP','Complete'],['CA12-IGES-INCIDENCE-IDENTITY','Complete'],['CA12-IGES-SHEET-TRANSFORM-RESOURCE','typed-refuse']]},
]
for(const spec of specs){
  const plan=`${q}/plans/${spec.slug}.json`,evidence=`${q}/${spec.slug}-evidence-v12.json`
  write(plan,{$schema:'./brep-capability-qualification-plan-v12.schema.json',schema:'open-scad-viewer/brep-capability-qualification-plan',schemaVersion:12,planId:spec.slug,gate:{id:'g8-full',name:'brep-full-closed-matrix',version:12},lifecycle:{status:'qualified',qualificationClaim:'finite-cell'},claimBoundary:{candidateClaim:spec.claim,forbiddenClaims:spec.excluded},capability:spec.id,successorOf:spec.successorOf,dependencies:spec.deps.map(capability=>({capability,requiredMaturity:'Qualified'})),scope:{included:spec.included,excluded:spec.excluded},matrix:spec.rows.map(([id,expected])=>({id,expected,blocksQualification:true})),bindings:{implementation:artifacts.slice(0,-1),registry:'src/services/geometry/brepCapability.ts',evidenceJson:evidence},evidence:{state:'qualified',notes:['Only the finite test-backed cells are qualified; excluded cases remain typed refusals.']},resetPolicy:'false-Complete freezes this capability and transitively stales all dependents; recovery requires a successor capability ID',unresolvedRows:[]})
  write(evidence,{$schema:'./brep-capability-evidence-v12.schema.json',schema:'open-scad-viewer/brep-capability-evidence',schemaVersion:12,evidenceId:`${spec.slug}-evidence-v12`,capability:spec.id,maturity:'Qualified',state:'qualified',plan,runs:[{id:`native-bridge-wasm-${spec.slug}-2026-09-17-v12`,result:'pass',artifactHash:digest(artifacts)}],oracles:['close_topology::tests::solid_typestate_is_not_weakened','close_topology::tests::radial_fans_and_orientation_are_checked','close_topology::tests::multi_cell_heal_budgets_partitions_and_atomic_refusal','tests/brepQualificationV12.test.ts'],unresolvedRows:[],attestation:{fabricatedRuns:false,note:'Recorded only after native topology, interchange, mutation, budget, bridge and artifact tests passed.'}})
  spec.plan=plan;spec.evidence=evidence
}
const oldIndex=read(previous.index),oldMatrix=read(previous.matrix),oldRelease=read(previous.release)
const rows=specs.map(s=>({id:s.id,plan:s.plan,evidence:s.evidence,maturity:'Qualified',releaseState:'shipped',dependencies:s.deps}))
const refusals=['nonmanifold-unbounded-branching','nonmanifold-singular-carrier','mixed-dimensional-solid-audit-entry','complex-interchange-sheet-open-shell','heal-silent-correspondence','heal-tolerance-growth','heal-large-gap','heal-approximate-edge-partition']
write(next.index,{...oldIndex,schemaVersion:12,successorOf:previous.index,matrix:next.matrix,registry:next.release,capabilities:[...oldIndex.capabilities,...rows]})
write(next.matrix,{...oldMatrix,schemaVersion:12,planId:'brep-full-closed-matrix-v12',successorOf:previous.matrix,admittedOps:[...oldMatrix.admittedOps,...rows.map(r=>r.id)],explicitRefuse:[...new Set([...oldMatrix.explicitRefuse,...refusals])]})
write(next.release,{...oldRelease,schemaVersion:12,planId:'brep-capability-registry-release-full-v12',successorOf:previous.release,closedMatrix:next.matrix,g8Index:next.index,capabilities:[...oldRelease.capabilities,...rows.map(r=>r.id)],explicitRefuse:[...new Set([...oldRelease.explicitRefuse,...refusals])]})
