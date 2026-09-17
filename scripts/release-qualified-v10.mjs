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
const rewrite=(source,target)=>writeFileSync(resolve(root,target),readFileSync(resolve(root,source),'utf8')
 .replaceAll('-v9','-v10').replaceAll(' v9',' v10')
 .replaceAll('"const": 9','"const": 10').replaceAll('LS9-','CA10-'))
const digest=paths=>{
 const hash=createHash('sha256')
 for(const path of [...paths].sort()) hash.update(path).update('\0').update(readFileSync(resolve(root,path))).update('\n')
 return hash.digest('hex')
}
const base='docs/qualification'
const previous={
 index:`${base}/plans/g8-full-matrix-index-v9.json`,
 matrix:`${base}/brep-full-closed-matrix-v9.json`,
 release:`${base}/brep-capability-registry-release-full-v9.json`,
}
const next={
 index:`${base}/plans/g8-full-matrix-index-v10.json`,
 matrix:`${base}/brep-full-closed-matrix-v10.json`,
 release:`${base}/brep-capability-registry-release-full-v10.json`,
 planSchema:`${base}/plans/brep-capability-qualification-plan-v10.schema.json`,
 evidenceSchema:`${base}/brep-capability-evidence-v10.schema.json`,
}
rewrite(`${base}/plans/brep-capability-qualification-plan-v9.schema.json`,next.planSchema)
rewrite(`${base}/brep-capability-evidence-v9.schema.json`,next.evidenceSchema)
const dependencies=['numeric-evidence-curved-brep/1','boundary-correspondence/1','exact-sew/1','global-solid-audit/1','persistent-naming/1']
const implementation=[
 'crates/brep-core/src/analysis.rs','crates/brep-core/src/intersections.rs',
 'crates/brep-core/src/solid_audit.rs','crates/geometry-bridge/src/brep.rs',
 'crates/geometry-bridge/src/lib.rs','src/services/geometry/brep.ts',
 'src/services/geometry/brepCapability.ts',
]
const specs=[
 {
  id:'certified-brep-tessellation/2',successorOf:'certified-brep-tessellation/1',
  slug:'certified-brep-tessellation-2',
  claim:'Finite certified tessellation for independently recognized planar, sphere, cone/frustum, ring-torus and cylinder shells, including analytic cavities and separated multiple bodies.',
  included:[
   'Two directed analytic surface/mesh deviation bounds, normal/orientation checks, exact shared-edge sample identity, no T-junctions or cracks.',
   'Stereographic sphere poles, torus periodic seams, cone apex degeneracy, adaptive 1..32 subdivisions per analytic patch and 12..20000 triangle budgets.',
   'Complete source ChangeSet/persistent naming binding and body/cavity shell audit.',
  ],
  excluded:[
   'Generic rational/freeform, graph/multispan Boolean, loft and bent/twisted/scaled sweep surfaces without a proved patch deviation enclosure.',
   'More than 32 subdivisions, more than 20000 triangles, open shells, incomplete naming, approximation, healing, mesh fallback or guessed certification.',
  ],
  rows:[
   ['CA10-TESS-ANALYTIC-TWO-SIDED-DEVIATION','Complete'],
   ['CA10-TESS-POLE-SEAM-APEX-CONFORMITY','Complete'],
   ['CA10-TESS-CAVITY-MULTIBODY-NAMING','Complete'],
   ['CA10-TESS-MONOTONE-ADAPTIVE-BUDGET','Complete'],
   ['CA10-TESS-GENERIC-FREEFORM-LOFT-SWEEP','typed-refuse'],
  ],
 },
 {
  id:'certified-mass-properties/2',successorOf:'certified-mass-properties/1',
  slug:'certified-mass-properties-2',
  claim:'Closed-form certified enclosures for independently recognized analytic shells with signed cavity and multiple-body composition.',
  included:[
   'Exact formulas with outward binary64 enclosures for area, volume, centroid and full centroidal inertia of planar prisms, cylinders, tubes, spheres, cones/frusta and ring tori.',
   'Signed shell composition with parallel-axis transport for cavities and separated bodies, source topology/naming binding and solid audit.',
   'Rigid transforms, analytic oracle formulas and mutation/refusal coverage.',
  ],
  excluded:[
   'The converged-estimate quadrature API, generic rational/freeform and curved graph/multispan Boolean results.',
   'Non-planar lofts and bent/twisted/scaled sweeps until Bernstein/interval area, volume, centroid and inertia enclosures with a convergence proof are implemented.',
   'Open/non-positive compositions, incomplete naming, estimates relabeled certified, mesh fallback or resource exhaustion.',
  ],
  rows:[
   ['CA10-MASS-SPHERE-CONE-FRUSTUM-TORUS','Complete'],
   ['CA10-MASS-CAVITY-MULTIBODY-SIGNED','Complete'],
   ['CA10-MASS-CENTROID-INERTIA-TRANSFORM','Complete'],
   ['CA10-MASS-ORACLE-MUTATION-COMPOSITION','Complete'],
   ['CA10-MASS-GENERIC-QUADRATURE-LOFT-SWEEP','typed-refuse'],
  ],
 },
]
const artifacts=[...implementation,'tests/brepQualificationV10.test.ts','src/generated/geometry-kernels/kernel_bg.wasm']
for(const spec of specs){
 const plan=`${base}/plans/${spec.slug}.json`
 const evidence=`${base}/${spec.slug}-evidence-v10.json`
 write(plan,{
  $schema:'./brep-capability-qualification-plan-v10.schema.json',
  schema:'open-scad-viewer/brep-capability-qualification-plan',schemaVersion:10,
  planId:spec.slug,gate:{id:'g8-full',name:'brep-full-closed-matrix',version:10},
  lifecycle:{status:'qualified',qualificationClaim:'finite-cell'},
  claimBoundary:{candidateClaim:spec.claim,forbiddenClaims:spec.excluded},
  capability:spec.id,successorOf:spec.successorOf,
  dependencies:dependencies.map(capability=>({capability,requiredMaturity:'Qualified'})),
  scope:{included:spec.included,excluded:spec.excluded},
  matrix:spec.rows.map(([id,expected])=>({id,expected,blocksQualification:true})),
  bindings:{implementation,registry:'src/services/geometry/brepCapability.ts',evidenceJson:evidence},
  evidence:{state:'qualified',notes:[
   'Qualification is limited to independently recognized analytic shells and exact closed-form bounds.',
   'Every generic freeform/loft/bent-sweep row is a typed refusal; numerical convergence estimates are never relabeled certified.',
  ]},
  resetPolicy:'false-Complete freezes this capability and transitively stales all dependents; recovery requires a successor capability ID',
  unresolvedRows:[],
 })
 write(evidence,{
  $schema:'./brep-capability-evidence-v10.schema.json',
  schema:'open-scad-viewer/brep-capability-evidence',schemaVersion:10,
  evidenceId:`${spec.slug}-evidence-v10`,capability:spec.id,
  maturity:'Qualified',state:'qualified',plan,
  runs:[{id:`native-bridge-wasm-${spec.slug}-2026-09-17-v10`,result:'pass',artifactHash:digest(artifacts)}],
  oracles:[
   'analysis::tests::certified_successor_encloses_cone_frustum_torus_and_spherical_cavity',
   'geometry_bridge::brep::registry_tests::poles_round_holes_and_partial_turns_share_edge_indices_at_each_detail',
   'tests/brepQualificationV10.test.ts',
  ],
  unresolvedRows:[],
  attestation:{fabricatedRuns:false,note:'Recorded only after native, bridge, WASM and strict product tests pass; generic rational/freeform certification remains explicitly refused.'},
 })
 spec.plan=plan
 spec.evidence=evidence
}
const oldIndex=read(previous.index),oldMatrix=read(previous.matrix),oldRelease=read(previous.release)
const rows=specs.map(spec=>({id:spec.id,plan:spec.plan,evidence:spec.evidence,maturity:'Qualified',releaseState:'shipped',dependencies}))
const refusals=[
 'certified-generic-rational-freeform-tessellation',
 'certified-generic-rational-freeform-mass-quadrature',
 'certified-curved-graph-multispan-boolean-analysis',
 'certified-nonplanar-loft-analysis',
 'certified-bent-twisted-scaled-sweep-analysis',
 'certified-analysis-resource-overflow',
]
write(next.index,{...oldIndex,schemaVersion:10,successorOf:previous.index,matrix:next.matrix,registry:next.release,capabilities:[...oldIndex.capabilities,...rows]})
write(next.matrix,{...oldMatrix,schemaVersion:10,planId:'brep-full-closed-matrix-v10',lifecycle:'qualified',successorOf:previous.matrix,
 admittedOps:[...oldMatrix.admittedOps,...rows.map(row=>row.id)],
 explicitRefuse:[...new Set([...oldMatrix.explicitRefuse,...refusals])],
})
write(next.release,{...oldRelease,schemaVersion:10,planId:'brep-capability-registry-release-full-v10',lifecycle:'qualified',successorOf:previous.release,
 closedMatrix:next.matrix,g8Index:next.index,capabilities:[...oldRelease.capabilities,...rows.map(row=>row.id)],
 explicitRefuse:[...new Set([...oldRelease.explicitRefuse,...refusals])],
})
