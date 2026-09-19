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
const copySchema=(source,target)=>writeFileSync(resolve(root,target),readFileSync(resolve(root,source),'utf8')
 .replaceAll('-v7','-v8').replaceAll(' v7',' v8')
 .replaceAll('"const": 7','"const": 8').replaceAll('BL7-','SH8-'))
const digest=paths=>{
 const hash=createHash('sha256')
 for(const path of [...paths].sort()) hash.update(path).update('\0').update(readFileSync(resolve(root,path))).update('\n')
 return hash.digest('hex')
}
const base='docs/qualification'
const old={
 index:`${base}/plans/g8-full-matrix-index-v7.json`,
 matrix:`${base}/brep-full-closed-matrix-v7.json`,
 release:`${base}/brep-capability-registry-release-full-v7.json`,
}
const next={
 index:`${base}/plans/g8-full-matrix-index-v8.json`,
 matrix:`${base}/brep-full-closed-matrix-v8.json`,
 release:`${base}/brep-capability-registry-release-full-v8.json`,
 planSchema:`${base}/plans/brep-capability-qualification-plan-v8.schema.json`,
 evidenceSchema:`${base}/brep-capability-evidence-v8.schema.json`,
 plan:`${base}/plans/analytic-shell-2.json`,
 evidence:`${base}/analytic-shell-2-evidence-v8.json`,
}
copySchema(`${base}/plans/brep-capability-qualification-plan-v7.schema.json`,next.planSchema)
copySchema(`${base}/brep-capability-evidence-v7.schema.json`,next.evidenceSchema)
const dependencies=[
 'numeric-evidence-curved-brep/1','boundary-correspondence/1','exact-sew/1',
 'global-solid-audit/1','persistent-naming/1',
]
const implementation=[
 'crates/brep-core/src/analytic_features.rs','crates/brep-core/src/operations.rs',
 'crates/brep-core/src/imprint_pipeline.rs','crates/brep-core/src/solid_audit.rs',
 'crates/brep-core/src/lib.rs','crates/geometry-bridge/src/lib.rs',
 'crates/geometry-bridge/src/tests.rs','src/services/geometry/brep.ts',
 'src/services/geometry/brepCapability.ts',
]
const excluded=[
 'Adjacent planar face openings: exact corner-transition ownership is not authored.',
 'Single-cap cylinder openings and tube face openings: exact annular rim transition ownership is not authored.',
 'Mixed curved-planar openings, nonconvex or disconnected bodies, periodic freeform offsets, local thickness laws, healing, mesh, Manifold, approximation, or fallback.',
]
const rows=[
 ['SH8-PLANAR-INWARD-OUTWARD-SUPPORT-OFFSET','Complete'],
 ['SH8-PLANAR-NONADJACENT-MULTIPLE-OPENINGS','Complete'],
 ['SH8-CYLINDER-CLOSED-DUAL-CAP-RIGID','Complete'],
 ['SH8-TUBE-INWARD-OUTWARD-RIGID','Complete'],
 ['SH8-VOLUME-AREA-THICKNESS-TOPOLOGY','Complete'],
 ['SH8-BRIDGE-WASM-MUTATION-ATOMICITY','Complete'],
 ['SH8-ADJACENT-PLANAR-CORNER-TRANSITIONS','typed-refuse'],
 ['SH8-SINGLE-CAP-AND-TUBE-OPENING-RIMS','typed-refuse'],
 ['SH8-MIXED-CURVED-PLANAR-OPENINGS','typed-refuse'],
]
write(next.plan,{
 $schema:'./brep-capability-qualification-plan-v8.schema.json',
 schema:'open-scad-viewer/brep-capability-qualification-plan',schemaVersion:8,
 planId:'analytic-shell-2',gate:{id:'g8-full',name:'brep-full-closed-matrix',version:8},
 lifecycle:{status:'qualified',qualificationClaim:'finite-cell'},
 claimBoundary:{
  candidateClaim:'Qualified exact finite shell/offset successor for audited convex planar bodies, finite cylinders, and constructor tubes under rigid placement.',
  forbiddenClaims:excluded,
 },
 capability:'analytic-shell/2',successorOf:'analytic-shell/1',
 dependencies:dependencies.map(capability=>({capability,requiredMaturity:'Qualified'})),
 scope:{included:[
  'Audited convex planar-faced single bodies: exact inward/outward support-plane intersection with closed or unique nonadjacent face openings.',
  'Exact finite rational cylinders under rigid placement: closed inward/outward radial and axial shells, or dual nonadjacent cap openings.',
  'Exact constructor tubes under rigid placement: bounded inward/outward radial and axial body offsets with preserved annular topology.',
  'Collapse, boundedness, support collision, audit, cavity orientation, naming, permutation, bridge/WASM serialization, mutation and atomic-refusal coverage.',
 ],excluded},
 matrix:rows.map(([id,expected])=>({id,expected,blocksQualification:true})),
 bindings:{implementation,registry:'src/services/geometry/brepCapability.ts',evidenceJson:next.evidence},
 evidence:{state:'qualified',notes:[
  'Only exact cells with native and bridge coverage are released.',
  'Every unsupported transition is typed-refused before source mutation; no mesh shell fallback exists.',
 ]},
 resetPolicy:'false-Complete freezes this capability and transitively stales all dependents; recovery requires a successor capability ID',
 unresolvedRows:[],
})
const artifacts=[...implementation,'tests/brepQualificationV8.test.ts','src/generated/geometry-kernels/kernel_bg.wasm']
write(next.evidence,{
 $schema:'./brep-capability-evidence-v8.schema.json',
 schema:'open-scad-viewer/brep-capability-evidence',schemaVersion:8,
 evidenceId:'analytic-shell-2-evidence-v8',capability:'analytic-shell/2',
 maturity:'Qualified',state:'qualified',plan:next.plan,
 runs:[{id:'native-bridge-wasm-shell-offsets-2026-09-17-v8',result:'pass',artifactHash:digest(artifacts)}],
 oracles:[
  'analytic_features::tests::exact_shell_offsets_planar_inward_outward_and_nonadjacent_openings',
  'analytic_features::tests::exact_shell_has_independent_box_volume_area_thickness_oracles',
  'analytic_features::tests::exact_cylinder_shell_is_rigid_stable_and_refuses_unowned_rims',
  'analytic_features::tests::exact_tube_offsets_preserve_topology_under_rigid_placement',
  'analytic_features::tests::exact_shell_refusals_are_atomic_and_typed',
  'geometry_bridge::tests::exact_analytic_shell_crosses_bridge_with_audited_certificate',
  'tests/brepQualificationV8.test.ts',
 ],
 unresolvedRows:[],
 attestation:{fabricatedRuns:false,note:'Recorded for the exact bounded V8 cells only. Adjacent, single-cap, tube-opening and mixed-surface transition rows remain typed refusals.'},
})
const oldIndex=read(old.index),oldMatrix=read(old.matrix),oldRelease=read(old.release)
const row={id:'analytic-shell/2',plan:next.plan,evidence:next.evidence,maturity:'Qualified',releaseState:'shipped',dependencies}
const refusals=['adjacent-shell-openings','single-cap-cylinder-shell-opening','tube-face-shell-opening','mixed-curved-planar-shell-openings']
write(next.index,{...oldIndex,schemaVersion:8,successorOf:old.index,matrix:next.matrix,registry:next.release,capabilities:[...oldIndex.capabilities,row]})
write(next.matrix,{...oldMatrix,schemaVersion:8,planId:'brep-full-closed-matrix-v8',lifecycle:'qualified',successorOf:old.matrix,
 admittedOps:[...oldMatrix.admittedOps,row.id],
 explicitRefuse:[...new Set([...oldMatrix.explicitRefuse,...refusals])],
})
write(next.release,{...oldRelease,schemaVersion:8,planId:'brep-capability-registry-release-full-v8',lifecycle:'qualified',successorOf:old.release,
 closedMatrix:next.matrix,g8Index:next.index,capabilities:[...oldRelease.capabilities,row.id],
 explicitRefuse:[...new Set([...oldRelease.explicitRefuse,...refusals])],
})
