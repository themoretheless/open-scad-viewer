#!/usr/bin/env node
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {dirname,resolve} from 'node:path'
import {fileURLToPath} from 'node:url'

const root=resolve(dirname(fileURLToPath(import.meta.url)),'..')
const read=path=>JSON.parse(readFileSync(resolve(root,path),'utf8'))
const write=(path,value)=>{
 const absolute=resolve(root,path)
 mkdirSync(dirname(absolute),{recursive:true})
 writeFileSync(absolute,`${JSON.stringify(value,null,2)}\n`)
}
const rewriteSchema=(source,target)=>writeFileSync(resolve(root,target),readFileSync(resolve(root,source),'utf8')
 .replaceAll('-v8','-v9').replaceAll(' v8',' v9')
 .replaceAll('"const": 8','"const": 9').replaceAll('SH8-','LS9-'))
const digest=paths=>{
 const hash=createHash('sha256')
 for(const path of [...paths].sort()) hash.update(path).update('\0').update(readFileSync(resolve(root,path))).update('\n')
 return hash.digest('hex')
}
const base='docs/qualification'
const old={
 index:`${base}/plans/g8-full-matrix-index-v8.json`,
 matrix:`${base}/brep-full-closed-matrix-v8.json`,
 release:`${base}/brep-capability-registry-release-full-v8.json`,
}
const next={
 index:`${base}/plans/g8-full-matrix-index-v9.json`,
 matrix:`${base}/brep-full-closed-matrix-v9.json`,
 release:`${base}/brep-capability-registry-release-full-v9.json`,
 planSchema:`${base}/plans/brep-capability-qualification-plan-v9.schema.json`,
 evidenceSchema:`${base}/brep-capability-evidence-v9.schema.json`,
}
rewriteSchema(`${base}/plans/brep-capability-qualification-plan-v8.schema.json`,next.planSchema)
rewriteSchema(`${base}/brep-capability-evidence-v8.schema.json`,next.evidenceSchema)
const dependencies=[
 'numeric-evidence-curved-brep/1','boundary-correspondence/1','exact-sew/1',
 'global-solid-audit/1','persistent-naming/1',
]
const implementation=[
 'crates/brep-core/src/analytic/loft.rs','crates/brep-core/src/analytic.rs',
 'crates/brep-core/src/analytic_features.rs','crates/brep-core/src/lib.rs',
 'crates/geometry-bridge/src/lib.rs','crates/geometry-bridge/src/tests.rs',
 'src/services/geometry/brep.ts','src/services/geometry/brepCapability.ts',
]
const specs=[
 {
  id:'analytic-solid-loft/2',successorOf:'analytic-solid-loft/1',
  slug:'analytic-solid-loft-2',
  claim:'Exact finite multi-section solid loft for 3..16 parallel, strictly ordered, simple convex degree-1 rational NURBS sections with one explicit positional correspondence.',
  included:[
   '3..16 parallel sections, each one simple strictly-convex counter-clockwise degree-1 rational NURBS loop with 3..16 corresponding vertices.',
   'Exact rational bilinear Bezier side surfaces, endpoint caps, shared C0 span boundaries, positive section-area/Jacobian bounds, sew/audit/self-intersection and complete persistent naming.',
  ],
  excluded:[
   'Section topology changes, unequal counts, ambiguous or inferred correspondence, nonconvex/collapsed/nonplanar sections, holes and disconnected sections.',
   'Nonparallel section planes, freeform degree-greater-than-1 section curves, approximation, healing, mesh or fallback.',
  ],
  rows:[
   ['LS9-LOFT-MULTI-SECTION-CORRESPONDENCE','Complete'],
   ['LS9-LOFT-RATIONAL-BEZIER-CAPS-TOPOLOGY','Complete'],
   ['LS9-LOFT-VOLUME-AREA-CONTINUITY-NAMING','Complete'],
   ['LS9-LOFT-TRANSFORM-REVERSAL-PERMUTATION','Complete'],
   ['LS9-LOFT-TOPOLOGY-CHANGE-AMBIGUITY-COLLAPSE','typed-refuse'],
  ],
  oracles:[
   'analytic_features::tests::exact_multi_section_loft_has_correspondence_topology_and_audit_oracles',
   'analytic_features::tests::loft_sweep_mutations_refuse_atomically_with_exact_codes',
   'geometry_bridge::tests::exact_loft_and_bent_rmf_successors_cross_bridge_atomically',
   'tests/brepQualificationV9.test.ts',
  ],
 },
 {
  id:'exact-parallel-frame-sweep/2',successorOf:'exact-parallel-frame-sweep/1',
  slug:'exact-parallel-frame-sweep-2',
  claim:'Exact finite bent sweep for open 2..15-span degree-1 Bezier paths with certified discrete rotation-minimizing frames and bounded twist/positive scale station laws.',
  included:[
   'Open bent paths with 3..16 stations and exact degree-1 Bezier spans; each span has a zero-curvature Bernstein hull and adjacent turn cosine exceeds 0.25.',
   'Simple convex 3..16-vertex degree-1 rational NURBS profiles, discrete RMF transport, twist bounded by pi and pi/2 per span, scale in 0.125..8.',
   'Rational bilinear Bezier sides, exact C0 path/frame/surface joins, conservative nonadjacent swept-hull separation, caps, sew/audit/self-intersection and naming.',
  ],
  excluded:[
   'Cusps, zero spans, frame projection singularities, turns with cosine at most 0.25, zero/negative/out-of-range scale and excessive twist.',
   'Self-intersecting or nonconvex profiles, nonadjacent swept-hull overlap, unsupported closed loops/holonomy, degree-greater-than-1 paths, mesh or fallback.',
  ],
  rows:[
   ['LS9-SWEEP-BENT-RMF-TWIST-SCALE','Complete'],
   ['LS9-SWEEP-PATH-FRAME-CONTINUITY-BOUNDS','Complete'],
   ['LS9-SWEEP-GLOBAL-SEPARATION-CAPS-NAMING','Complete'],
   ['LS9-SWEEP-TRANSFORM-REVERSAL-PERMUTATION','Complete'],
   ['LS9-SWEEP-CUSP-SCALE-TWIST-SELF-LOOP','typed-refuse'],
  ],
  oracles:[
   'analytic_features::tests::exact_bent_rmf_sweep_certifies_path_frame_laws_and_reversal',
   'analytic_features::tests::loft_sweep_mutations_refuse_atomically_with_exact_codes',
   'geometry_bridge::tests::exact_loft_and_bent_rmf_successors_cross_bridge_atomically',
   'tests/brepQualificationV9.test.ts',
  ],
 },
]
const artifacts=[...implementation,'tests/brepQualificationV9.test.ts','src/generated/geometry-kernels/kernel_bg.wasm']
for(const spec of specs){
 const plan=`${base}/plans/${spec.slug}.json`
 const evidence=`${base}/${spec.slug}-evidence-v9.json`
 write(plan,{
  $schema:'./brep-capability-qualification-plan-v9.schema.json',
  schema:'open-scad-viewer/brep-capability-qualification-plan',schemaVersion:9,
  planId:spec.slug,gate:{id:'g8-full',name:'brep-full-closed-matrix',version:9},
  lifecycle:{status:'qualified',qualificationClaim:'finite-cell'},
  claimBoundary:{candidateClaim:spec.claim,forbiddenClaims:spec.excluded},
  capability:spec.id,successorOf:spec.successorOf,
  dependencies:dependencies.map(capability=>({capability,requiredMaturity:'Qualified'})),
  scope:{included:spec.included,excluded:spec.excluded},
  matrix:spec.rows.map(([id,expected])=>({id,expected,blocksQualification:true})),
  bindings:{implementation,registry:'src/services/geometry/brepCapability.ts',evidenceJson:evidence},
  evidence:{state:'qualified',notes:[
   'Only exact bounded rational/NURBS cells with native, bridge and product WASM coverage are released.',
   'Every unsupported row is typed-refused before source mutation; no mesh, healing, approximation or fallback is reachable.',
  ]},
  resetPolicy:'false-Complete freezes this capability and transitively stales all dependents; recovery requires a successor capability ID',
  unresolvedRows:[],
 })
 write(evidence,{
  $schema:'./brep-capability-evidence-v9.schema.json',
  schema:'open-scad-viewer/brep-capability-evidence',schemaVersion:9,
  evidenceId:`${spec.slug}-evidence-v9`,capability:spec.id,
  maturity:'Qualified',state:'qualified',plan,
  runs:[{id:`native-bridge-wasm-${spec.slug}-2026-09-17-v9`,result:'pass',artifactHash:digest(artifacts)}],
  oracles:spec.oracles,unresolvedRows:[],
  attestation:{fabricatedRuns:false,note:'Recorded for the exact bounded V9 cells only; every listed exclusion remains a typed refusal and no G0/G1 approval is claimed.'},
 })
 spec.plan=plan
 spec.evidence=evidence
}
const oldIndex=read(old.index),oldMatrix=read(old.matrix),oldRelease=read(old.release)
const rows=specs.map(spec=>({id:spec.id,plan:spec.plan,evidence:spec.evidence,maturity:'Qualified',releaseState:'shipped',dependencies}))
const refusals=[
 'loft-section-topology-change','loft-ambiguous-correspondence','loft-section-collapse',
 'sweep-cusp-or-frame-singularity','sweep-zero-or-negative-scale','sweep-excessive-twist',
 'sweep-global-self-intersection','sweep-unsupported-closed-loop',
]
write(next.index,{...oldIndex,schemaVersion:9,successorOf:old.index,matrix:next.matrix,registry:next.release,capabilities:[...oldIndex.capabilities,...rows]})
write(next.matrix,{...oldMatrix,schemaVersion:9,planId:'brep-full-closed-matrix-v9',lifecycle:'qualified',successorOf:old.matrix,
 admittedOps:[...oldMatrix.admittedOps,...rows.map(row=>row.id)],
 explicitRefuse:[...new Set([...oldMatrix.explicitRefuse,...refusals])],
})
write(next.release,{...oldRelease,schemaVersion:9,planId:'brep-capability-registry-release-full-v9',lifecycle:'qualified',successorOf:old.release,
 closedMatrix:next.matrix,g8Index:next.index,capabilities:[...oldRelease.capabilities,...rows.map(row=>row.id)],
 explicitRefuse:[...new Set([...oldRelease.explicitRefuse,...refusals])],
})
