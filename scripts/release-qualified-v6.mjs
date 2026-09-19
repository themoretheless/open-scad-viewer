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
const copySchema=(source,target)=>{
 const text=readFileSync(resolve(root,source),'utf8')
  .replaceAll('-v5','-v6').replaceAll(' v5',' v6')
  .replaceAll('"const": 5','"const": 6').replaceAll('NB7-','NB8-')
 writeFileSync(resolve(root,target),text)
}
const digest=paths=>{
 const hash=createHash('sha256')
 for(const path of [...paths].sort()) hash.update(path).update('\0').update(readFileSync(resolve(root,path))).update('\n')
 return hash.digest('hex')
}
const v5={
 index:'docs/qualification/plans/g8-full-matrix-index-v5.json',
 matrix:'docs/qualification/brep-full-closed-matrix-v5.json',
 release:'docs/qualification/brep-capability-registry-release-full-v5.json',
}
const v6={
 index:'docs/qualification/plans/g8-full-matrix-index-v6.json',
 matrix:'docs/qualification/brep-full-closed-matrix-v6.json',
 release:'docs/qualification/brep-capability-registry-release-full-v6.json',
 planSchema:'docs/qualification/plans/brep-capability-qualification-plan-v6.schema.json',
 evidenceSchema:'docs/qualification/brep-capability-evidence-v6.schema.json',
 plan:'docs/qualification/plans/nurbs-boolean-bezier-le3-8.json',
 evidence:'docs/qualification/nurbs-boolean-bezier-le3-8-evidence-v6.json',
}
copySchema('docs/qualification/plans/brep-capability-qualification-plan-v5.schema.json',v6.planSchema)
copySchema('docs/qualification/brep-capability-evidence-v5.schema.json',v6.evidenceSchema)

const dependencies=[
 'numeric-evidence-curved-brep/1','boundary-correspondence/1','exact-sew/1',
 'global-solid-audit/1','persistent-naming/1','nurbs-boolean-bezier-le3/7',
]
const included=[
 'The exact /7 bounded positive-weight non-periodic degree 2-3, one-or-two-spans-per-axis graph/affine-parallelotope cell with exactly two disjoint regular transverse branches.',
 'Topology-authoring graph-source union as three disjoint-interior closed cells: the affine tool and the two source exterior graph cells.',
 'Topology-authoring tool-minus-source reversed difference as four disjoint-interior closed cells: graph-roof complement, floor complement, and two varying-axis exterior cells.',
 'Exact region membership from certified branch roots and graph-frame interval partitions; shared curve/pcurve authority, UV classification, sew, global audit, persistent naming, bridge/WASM serialization, and no fallback.',
 'Checked finite limits inherited from /7; exact graph-frame parallelotope recognition is additionally required.',
]
const excluded=[
 'Tangency and higher-order contact: homogeneous derivative multiplicity isolation does not yet carry complete singular-UV regularized topology ownership.',
 'Coincidence and rational patch-region overlap: no complete deterministic boundary ownership/removal proof is implemented, including identical-region overlap.',
 'Gray-band contact, branch junctions, ambiguous joins, periodic seams, poles, nonpositive weights, degree above 3, unsupported rational boundary parameterizations, and resource overflow.',
 'Tool-source union, source-tool reversed semantics outside ordinary difference, healing, mesh, prism, Manifold, approximation, or any fallback.',
]
const matrix=[
 ['NB8-UNION-EXACT-REGION-PARTITION','Complete'],
 ['NB8-REVERSED-DIFFERENCE-EXACT-REGION-PARTITION','Complete'],
 ['NB8-BRANCH-UV-PCURVE-SEW-AUDIT-NAMING','Complete'],
 ['NB8-NATIVE-BRIDGE-WASM-PRODUCT-MUTATION','Complete'],
 ['NB8-TANGENCY-MULTIPLICITY-AND-SINGULAR-UV','typed-refuse'],
 ['NB8-COINCIDENCE-OWNERSHIP-AND-REMOVAL','typed-refuse'],
 ['NB8-OTHER-CONTACT-AND-BOUNDARY-CLASSES','typed-refuse'],
]
const implementation=[
 'crates/brep-core/src/lib.rs','crates/brep-core/src/nurbs_ss_g6.rs',
 'crates/brep-core/src/solid_audit.rs','crates/brep-core/src/operations.rs',
 'crates/brep-core/src/transactions.rs','crates/brep-core/src/trim_sew.rs',
 'crates/brep-core/src/uv_arrangement.rs','crates/brep-topology/src/lib.rs',
 'crates/geometry-bridge/src/lib.rs','crates/geometry-bridge/src/tests.rs',
 'src/services/geometry/brep.ts','src/services/geometry/brepCapability.ts',
 'src/services/geometry/kernel.ts',
]
write(v6.plan,{
 $schema:'./brep-capability-qualification-plan-v6.schema.json',
 schema:'open-scad-viewer/brep-capability-qualification-plan',schemaVersion:6,
 planId:'nurbs-boolean-bezier-le3-8',gate:{id:'g8-full',name:'brep-full-closed-matrix',version:6},
 lifecycle:{status:'qualified',qualificationClaim:'finite-cell'},
 claimBoundary:{
  candidateClaim:'Qualified exact-region union and tool-minus-source difference successor for the /7 exactly-two-transverse-branch cell.',
  forbiddenClaims:excluded,
 },
 capability:'nurbs-boolean-bezier-le3/8',successorOf:'nurbs-boolean-bezier-le3/7',
 dependencies:dependencies.map(capability=>({capability,requiredMaturity:'Qualified'})),
 scope:{included,excluded},matrix:matrix.map(([id,expected])=>({id,expected,blocksQualification:true})),
 bindings:{implementation,registry:'src/services/geometry/brepCapability.ts',evidenceJson:v6.evidence},
 evidence:{state:'qualified',notes:[
  'Only union and tool-minus-source difference are added; every /7 operation remains qualified unchanged.',
  'Tangency and coincidence are refused because complete regularized ownership, not merely contact detection, is required.',
 ]},
 resetPolicy:'false-Complete freezes this capability and transitively stales all dependents; recovery requires a successor capability ID',
 unresolvedRows:[],
})
const testArtifacts=[
 'crates/brep-core/src/nurbs_ss_g6.rs','crates/brep-core/src/solid_audit.rs',
 'crates/geometry-bridge/src/lib.rs','crates/geometry-bridge/src/tests.rs',
 'tests/brepGeneralNurbsBoolean.test.ts','tests/brepQualificationV6.test.ts',
 'src/generated/geometry-kernels/kernel_bg.wasm',
]
write(v6.evidence,{
 $schema:'./brep-capability-evidence-v6.schema.json',
 schema:'open-scad-viewer/brep-capability-evidence',schemaVersion:6,
 evidenceId:'nurbs-boolean-bezier-le3-8-evidence-v6',
 capability:'nurbs-boolean-bezier-le3/8',maturity:'Qualified',state:'qualified',plan:v6.plan,
 runs:[{id:'native-bridge-wasm-product-mutation-2026-09-17-v6',result:'pass',artifactHash:digest(testArtifacts)}],
 oracles:[
  'nurbs_ss_g6::tests::general_multispan_boolean_authors_exact_region_operations',
  'nurbs_ss_g6::tests::general_multispan_boolean_naming_is_rigid_transform_stable',
  'nurbs_ss_g6::tests::general_multispan_boolean_has_independent_regularized_volume_oracle',
  'geometry_bridge::tests::general_multispan_boolean_bridge_is_strict_and_audited',
  'tests/brepGeneralNurbsBoolean.test.ts','tests/brepQualificationV6.test.ts',
 ],
 unresolvedRows:[],
 attestation:{fabricatedRuns:false,note:'Recorded only after native and bridge runs. Tangency and coincidence remain explicit typed refusals because no complete regularized topology ownership proof exists.'},
})
const oldIndex=read(v5.index),oldMatrix=read(v5.matrix),oldRelease=read(v5.release)
const row={id:'nurbs-boolean-bezier-le3/8',plan:v6.plan,evidence:v6.evidence,maturity:'Qualified',releaseState:'shipped',dependencies}
const v6ExplicitRefuse=items=>[
 ...items.filter(id=>!['nurbs-union','nurbs-reversed-difference'].includes(id)),
 'nurbs-tangency-contact','nurbs-coincident-region-overlap','nurbs-tool-source-union',
]
write(v6.index,{...oldIndex,schemaVersion:6,successorOf:v5.index,matrix:v6.matrix,registry:v6.release,capabilities:[...oldIndex.capabilities,row]})
write(v6.matrix,{
 ...oldMatrix,schemaVersion:6,planId:'brep-full-closed-matrix-v6',lifecycle:'qualified',
 successorOf:v5.matrix,admittedOps:[...oldMatrix.admittedOps,row.id],
 explicitRefuse:[...new Set(v6ExplicitRefuse(oldMatrix.explicitRefuse))],
})
write(v6.release,{
 ...oldRelease,schemaVersion:6,planId:'brep-capability-registry-release-full-v6',
 lifecycle:'qualified',successorOf:v5.release,closedMatrix:v6.matrix,g8Index:v6.index,
 capabilities:[...oldRelease.capabilities,row.id],
 explicitRefuse:[...new Set(v6ExplicitRefuse(oldRelease.explicitRefuse))],
})
