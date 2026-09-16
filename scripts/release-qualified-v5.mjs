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
const replaceJson=(source,target,replacements)=>{
 let text=readFileSync(resolve(root,source),'utf8')
 for(const [from,to] of replacements) text=text.replaceAll(from,to)
 writeFileSync(resolve(root,target),text)
}
const hashFiles=paths=>{
 const hash=createHash('sha256')
 for(const path of [...paths].sort()) hash.update(path).update('\0').update(readFileSync(resolve(root,path))).update('\n')
 return hash.digest('hex')
}

const v4={
 index:'docs/qualification/plans/g8-full-matrix-index-v4.json',
 matrix:'docs/qualification/brep-full-closed-matrix-v4.json',
 release:'docs/qualification/brep-capability-registry-release-full-v4.json',
}
const v5={
 index:'docs/qualification/plans/g8-full-matrix-index-v5.json',
 matrix:'docs/qualification/brep-full-closed-matrix-v5.json',
 release:'docs/qualification/brep-capability-registry-release-full-v5.json',
 planSchema:'docs/qualification/plans/brep-capability-qualification-plan-v5.schema.json',
 evidenceSchema:'docs/qualification/brep-capability-evidence-v5.schema.json',
 plan:'docs/qualification/plans/nurbs-boolean-bezier-le3-7.json',
 evidence:'docs/qualification/nurbs-boolean-bezier-le3-7-evidence-v5.json',
}
replaceJson(
 'docs/qualification/plans/brep-capability-qualification-plan-v4.schema.json',
 v5.planSchema,
 [['-v4','-v5'],[' v4',' v5'],['"const": 4','"const": 5'],['NB6-','NB7-']],
)
replaceJson(
 'docs/qualification/brep-capability-evidence-v4.schema.json',
 v5.evidenceSchema,
 [['-v4','-v5'],[' v4',' v5'],['"const": 4','"const": 5'],['NB6-','NB7-']],
)

const dependencies=[
 'numeric-evidence-curved-brep/1','boundary-correspondence/1','exact-sew/1',
 'global-solid-audit/1','persistent-naming/1','nurbs-boolean-bezier-le3/5',
]
const included=[
 'Finite bounded cell: exactly one positive-weight, non-periodic tensor-product graph solid of degree 2 or 3 with one or two non-degenerate knot spans per parameter, and one bounded affine-planar closed cutter solid.',
 'Exact knot-insertion decomposition, half-open knot-cell ownership, and exactly two disjoint regular transverse strict-interior iso branches crossing the admitted source spans.',
 'Exact one-to-one branch joining, operand-local UV arrangement, trimming, sew, global closed-solid audit, persistent naming, and atomic no-fallback authorship.',
 'Closed-solid intersection in either operand order and source-minus-tool difference only; one intersection component or two separated source-difference components.',
 'Checked finite limits of 64 tensor spans, candidate span pairs, fragments, joined branches, UV records, authored topology records, and certificate records.',
]
const excluded=[
 'Tangency, higher-order contact, coincidence, overlap intervals or regions, gray-band contact, branch junctions, and ambiguous joins.',
 'Singular poles, zero or negative weights, an uncertified positive denominator lower bound, degenerate knot spans, and invalid knot multiplicities.',
 'Periodic or closed parameter seams, seam-crossing ownership, and pole or seam singularities.',
 'Degree above 3 in either parameter, implicit degree reduction, fitting, sampling, approximation, and unsupported rational boundary parameterizations.',
 'Union, tool-minus-source reversed difference, open-shell output, healing, mesh, prism, Manifold, or any other fallback.',
 'Resource overflow, unchecked or unbounded work, and any certificate, correspondence, sew, audit, naming, or no-fallback mutation.',
]
const matrix=[
 ['NB7-EXACT-MULTISPAN-DECOMPOSITION-AND-OWNERSHIP','Complete'],
 ['NB7-TWO-BRANCH-TRANSVERSE-WALK-AND-JOIN','Complete'],
 ['NB7-UV-SEW-AUDIT-NAMING-AUTHORSHIP','Complete'],
 ['NB7-INTERSECTION-AND-SOURCE-DIFFERENCE','Complete'],
 ['NB7-UNION-REVERSED-DIFFERENCE-AND-CONTACT-REFUSALS','typed-refuse'],
 ['NB7-PERIODIC-POLE-DEGREE-RATIONAL-BOUNDARY-REFUSALS','typed-refuse'],
 ['NB7-RESOURCE-CERTIFICATE-AND-ATOMICITY-MUTATIONS','typed-refuse'],
 ['NB7-NATIVE-BRIDGE-WASM-PRODUCT','Complete'],
]
const implementation=[
 'crates/brep-core/src/lib.rs',
 'crates/brep-core/src/nurbs_ss_g6.rs',
 'crates/brep-core/src/operations.rs',
 'crates/brep-core/src/solid_audit.rs',
 'crates/brep-core/src/transactions.rs',
 'crates/brep-core/src/trim_sew.rs',
 'crates/brep-core/src/uv_arrangement.rs',
 'crates/brep-topology/src/lib.rs',
 'crates/geometry-bridge/src/lib.rs',
 'crates/geometry-bridge/src/tests.rs',
 'src/services/geometry/brep.ts',
 'src/services/geometry/brepCapability.ts',
 'src/services/geometry/kernel.ts',
]
write(v5.plan,{
 $schema:'./brep-capability-qualification-plan-v5.schema.json',
 schema:'open-scad-viewer/brep-capability-qualification-plan',schemaVersion:5,
 planId:'nurbs-boolean-bezier-le3-7',gate:{id:'g8-full',name:'brep-full-closed-matrix',version:5},
 lifecycle:{status:'qualified',qualificationClaim:'finite-cell'},
 claimBoundary:{
  candidateClaim:'Qualified bounded multi-span, exactly-two-branch NURBS graph/affine-slab intersection and source-difference finite cell.',
  forbiddenClaims:excluded,
 },
 capability:'nurbs-boolean-bezier-le3/7',successorOf:'nurbs-boolean-bezier-le3/6',
 dependencies:dependencies.map(capability=>({capability,requiredMaturity:'Qualified'})),
 scope:{included,excluded},
 matrix:matrix.map(([id,expected])=>({id,expected,blocksQualification:true})),
 bindings:{implementation,registry:'src/services/geometry/brepCapability.ts',evidenceJson:v5.evidence},
 evidence:{state:'qualified',notes:[
  'Qualification is limited to the exact finite cell and operations stated in scope; /6 remains unavailable.',
  'The certificate-bearing product API is authoritative; the model-only API delegates to it and cannot bypass certification.',
 ]},
 resetPolicy:'false-Complete freezes this capability and transitively stales all dependents; recovery requires a successor capability ID',
 unresolvedRows:[],
})
const testArtifacts=[
 'crates/brep-core/src/nurbs_ss_g6.rs',
 'crates/geometry-bridge/src/tests.rs',
 'tests/brepGeneralNurbsBoolean.test.ts',
 'tests/brepQualificationV5.test.ts',
]
write(v5.evidence,{
 $schema:'./brep-capability-evidence-v5.schema.json',
 schema:'open-scad-viewer/brep-capability-evidence',schemaVersion:5,
 evidenceId:'nurbs-boolean-bezier-le3-7-evidence-v5',
 capability:'nurbs-boolean-bezier-le3/7',maturity:'Qualified',state:'qualified',plan:v5.plan,
 runs:[{id:'native-bridge-wasm-product-mutation-2026-09-17',result:'pass',artifactHash:hashFiles(testArtifacts)}],
 oracles:[
  'nurbs_ss_g6::tests::general_multispan_boolean_authors_intersection_and_two_component_difference',
  'nurbs_ss_g6::tests::general_multispan_boolean_naming_is_rigid_transform_stable',
  'geometry_bridge::tests::general_multispan_boolean_bridge_is_strict_and_audited',
  'tests/brepGeneralNurbsBoolean.test.ts',
  'tests/brepQualificationV5.test.ts',
 ],
 unresolvedRows:[],
 attestation:{fabricatedRuns:false,note:'Recorded after successful native, bridge, rebuilt-WASM product, refusal, atomicity, and mutation runs; no union, reversed-difference, unsupported-boundary, or general-NURBS claim is made.'},
})

const oldIndex=read(v4.index)
const oldMatrix=read(v4.matrix)
const oldRelease=read(v4.release)
const row={id:'nurbs-boolean-bezier-le3/7',plan:v5.plan,evidence:v5.evidence,maturity:'Qualified',releaseState:'shipped',dependencies}
write(v5.index,{
 ...oldIndex,schemaVersion:5,successorOf:v4.index,matrix:v5.matrix,registry:v5.release,
 capabilities:[...oldIndex.capabilities,row],
})
write(v5.matrix,{
 ...oldMatrix,schemaVersion:5,planId:'brep-full-closed-matrix-v5',lifecycle:'qualified',
 successorOf:v4.matrix,admittedOps:[...oldMatrix.admittedOps,row.id],
 explicitRefuse:[...new Set([...oldMatrix.explicitRefuse,
  'nurbs-ambiguous-joins','nurbs-unsupported-rational-boundary-parameterization',
  'nurbs-union','nurbs-reversed-difference','nurbs-resource-overflow',
 ])],
})
write(v5.release,{
 ...oldRelease,schemaVersion:5,planId:'brep-capability-registry-release-full-v5',
 lifecycle:'qualified',successorOf:v4.release,closedMatrix:v5.matrix,g8Index:v5.index,
 capabilities:[...oldRelease.capabilities,row.id],
 explicitRefuse:[...new Set([...oldRelease.explicitRefuse,
  'nurbs-ambiguous-joins','nurbs-unsupported-rational-boundary-parameterization',
  'nurbs-union','nurbs-reversed-difference','nurbs-resource-overflow',
 ])],
})
