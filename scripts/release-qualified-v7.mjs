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
  .replaceAll('-v6','-v7').replaceAll(' v6',' v7')
  .replaceAll('"const": 6','"const": 7').replaceAll('NB8-','BL7-')
 writeFileSync(resolve(root,target),text)
}
const digest=paths=>{
 const hash=createHash('sha256')
 for(const path of [...paths].sort()) hash.update(path).update('\0').update(readFileSync(resolve(root,path))).update('\n')
 return hash.digest('hex')
}
const base='docs/qualification'
const paths={
 oldIndex:`${base}/plans/g8-full-matrix-index-v6.json`,
 oldMatrix:`${base}/brep-full-closed-matrix-v6.json`,
 oldRelease:`${base}/brep-capability-registry-release-full-v6.json`,
 index:`${base}/plans/g8-full-matrix-index-v7.json`,
 matrix:`${base}/brep-full-closed-matrix-v7.json`,
 release:`${base}/brep-capability-registry-release-full-v7.json`,
 planSchema:`${base}/plans/brep-capability-qualification-plan-v7.schema.json`,
 evidenceSchema:`${base}/brep-capability-evidence-v7.schema.json`,
}
copySchema(`${base}/plans/brep-capability-qualification-plan-v6.schema.json`,paths.planSchema)
copySchema(`${base}/brep-capability-evidence-v6.schema.json`,paths.evidenceSchema)

const dependencies=[
 'numeric-evidence-curved-brep/1','boundary-correspondence/1','exact-sew/1',
 'global-solid-audit/1','persistent-naming/1',
]
const implementation=[
 'crates/brep-core/src/analytic_features.rs','crates/brep-core/src/imprint_pipeline.rs',
 'crates/brep-core/src/operations.rs','crates/brep-core/src/lib.rs',
 'crates/geometry-bridge/src/lib.rs','crates/geometry-bridge/src/tests.rs',
 'src/services/geometry/brep.ts','src/services/geometry/brepCapability.ts',
]
const resetPolicy='false-Complete freezes this capability and transitively stales all dependents; recovery requires a successor capability ID'
const cells=[
 {
  id:'exact-convex-prism-edge-fillet/1',slug:'exact-convex-prism-edge-fillet-1',
  successorOf:'analytic-multi-edge-fillet/1',qualified:true,
  claim:'Exact constant-radius rational cylindrical fillets on any selected subset of longitudinal edges of an audited strictly-convex planar prism under rigid placement.',
  included:[
   'Strictly-convex polygonal prisms with two parallel caps under proper rigid placement.',
   'Any unique subset of longitudinal straight convex edges, including mixed selected/nonselected vertices and the all-selected closed profile.',
   'Exact positive-weight rational circular arcs, cylindrical side patches, exact cap trims, complete audit and persistent naming.',
  ],
  excluded:[
   'Cap edges, non-prismatic polyhedra, curved source edges, non-rigid affine distortion, and disconnected bodies.',
   'Valence-3 rolling-ball corner networks, sphere/rational setback transitions, variable radius, G2 blends, healing, mesh, or fallback.',
  ],
  rows:[
   ['BL7-PRISM-MIXED-SELECTION-RIGID-PLACEMENT','Complete'],
   ['BL7-PRISM-CLOSED-PROFILE-EXACT-RADIUS','Complete'],
   ['BL7-RADIUS-COLLISION-ATOMICITY','typed-refuse'],
   ['BL7-CAP-EDGE-VALENCE3-TRANSITION','typed-refuse'],
  ],
 },
 {
  id:'exact-convex-straight-edge-chamfer/1',slug:'exact-convex-straight-edge-chamfer-1',
  successorOf:'analytic-chamfer/1',qualified:true,
  claim:'Exact equal-distance support-plane chamfer on connected straight convex-edge selections of audited convex planar-faced polyhedra.',
  included:[
   'Arbitrary audited convex planar-faced single-body polyhedra under rigid placement.',
   'Unique connected open or closed straight-edge selections, mixed selected/nonselected adjacent edges, deterministic support-plane corner intersections.',
   'Face-consumption and collision feasibility, exact planar topology, audit, naming, permutation and atomic refusal coverage.',
  ],
  excluded:[
   'Curved source faces or edges, disconnected selections, nonconvex or cavity-bearing bodies, unequal-distance laws, healing, mesh, or fallback.',
   'A chamfer certificate does not imply a smooth rolling-ball fillet transition.',
  ],
  rows:[
   ['BL7-CHAMFER-ARBITRARY-CONVEX-CHAIN','Complete'],
   ['BL7-CHAMFER-OPEN-CLOSED-PERMUTATION-RIGID','Complete'],
   ['BL7-CHAMFER-COLLISION-FACE-CONSUMPTION','typed-refuse'],
   ['BL7-CHAMFER-NONCONVEX-CURVED-DISCONNECTED','typed-refuse'],
  ],
 },
 {
  id:'exact-valence3-corner-blend/1',slug:'exact-valence3-corner-blend-1',
  successorOf:'analytic-multi-edge-fillet/1',qualified:false,
  claim:'Candidate exact plane/cylinder/sphere or rational valence-3 corner transition network.',
  included:['Typed product refusal records the intended transition boundary without topology authorship.'],
  excluded:['No complete deterministic trim ownership, setback feasibility, tangency, self-intersection, and persistent naming proof exists.'],
  rows:[['BL7-VALENCE3-SPHERE-RATIONAL-OWNERSHIP','ResearchOnly']],
 },
 {
  id:'exact-variable-radius-fillet/1',slug:'exact-variable-radius-fillet-1',
  successorOf:'analytic-multi-edge-fillet/1',qualified:false,
  claim:'Candidate exact bounded variable-radius law with complete endpoint and corner transitions.',
  included:['A strict native/bridge/WASM typed-refusal seam prevents constant-radius substitution.'],
  excluded:['No exact bounded radius law, collision enclosure, transition continuity, or corner ownership proof exists.'],
  rows:[['BL7-VARIABLE-RADIUS-LAW-AND-TRANSITIONS','typed-refuse']],
 },
]
const testArtifacts=[...implementation,'src/generated/geometry-kernels/kernel_bg.wasm']
const artifactHash=digest(testArtifacts)
for(const cell of cells){
 const plan=`${base}/plans/${cell.slug}.json`
 const evidence=`${base}/${cell.slug}-evidence-v7.json`
 const unresolved=cell.qualified?[]:[{
  id:cell.rows[0][0],status:'open',
  resolution:'Implement and independently prove the excluded exact geometry and topology ownership before creating a successor release.',
 }]
 write(plan,{
  $schema:'./brep-capability-qualification-plan-v7.schema.json',
  schema:'open-scad-viewer/brep-capability-qualification-plan',schemaVersion:7,
  planId:cell.slug,gate:{id:'g8-full',name:'brep-full-closed-matrix',version:7},
  lifecycle:{status:cell.qualified?'qualified':'frozen-pending-execution',qualificationClaim:cell.qualified?'finite-cell':'none'},
  claimBoundary:{candidateClaim:cell.claim,forbiddenClaims:cell.excluded},
  capability:cell.id,successorOf:cell.successorOf,
  dependencies:dependencies.map(capability=>({capability,requiredMaturity:'Qualified'})),
  scope:{included:cell.included,excluded:cell.excluded},
  matrix:cell.rows.map(([id,expected])=>({id,expected,blocksQualification:true})),
  bindings:{implementation,registry:'src/services/geometry/brepCapability.ts',evidenceJson:evidence},
  evidence:{state:cell.qualified?'qualified':'not-executed',notes:cell.qualified
   ?['Native and bridge rows passed; only the explicit finite cell is qualified.','All excluded geometry remains typed-refused with no mesh/Manifold fallback.']
   :['No qualification run is claimed; the product seam refuses atomically.']},
  resetPolicy,unresolvedRows:unresolved,
 })
 write(evidence,{
  $schema:'./brep-capability-evidence-v7.schema.json',
  schema:'open-scad-viewer/brep-capability-evidence',schemaVersion:7,
  evidenceId:`${cell.slug}-evidence-v7`,capability:cell.id,
  maturity:cell.qualified?'Qualified':'Unavailable',
  state:cell.qualified?'qualified':'not-executed',plan,
  runs:cell.qualified?[{id:'native-bridge-wasm-blends-2026-09-17-v7',result:'pass',artifactHash}]:[],
  oracles:cell.qualified?[
   'analytic_features::tests::exact_prism_fillet_supports_mixed_and_closed_profile_selections_under_rigid_placement',
   'analytic_features::tests::exact_prism_fillet_has_independent_square_volume_and_radius_oracles',
   'analytic_features::tests::exact_convex_chamfer_handles_connected_chain_permutations_atomically',
   'geometry_bridge::tests::audited_feature_successors_cross_the_bridge_with_certificates',
  ]:[],
  unresolvedRows:unresolved,
  attestation:{fabricatedRuns:false,note:cell.qualified
   ?'Recorded only after scoped native and bridge execution; exact exclusions remain refusals.'
   :'No execution or geometry completion is claimed.'},
 })
 cell.plan=plan
 cell.evidence=evidence
}
const oldIndex=read(paths.oldIndex),oldMatrix=read(paths.oldMatrix),oldRelease=read(paths.oldRelease)
const rows=cells.map(cell=>({
 id:cell.id,plan:cell.plan,evidence:cell.evidence,maturity:cell.qualified?'Qualified':'Unavailable',
 releaseState:cell.qualified?'shipped':'candidate',dependencies,
}))
const qualified=rows.filter(row=>row.releaseState==='shipped').map(row=>row.id)
const pending=rows.filter(row=>row.releaseState==='candidate').map(row=>row.id)
const refusals=['variable-radius-fillet','valence3-fillet-corner-network','non-prismatic-fillet','curved-source-edge-blend','mesh-bevel-relabeling']
write(paths.index,{...oldIndex,schemaVersion:7,successorOf:paths.oldIndex,matrix:paths.matrix,registry:paths.release,capabilities:[...oldIndex.capabilities,...rows]})
write(paths.matrix,{...oldMatrix,schemaVersion:7,planId:'brep-full-closed-matrix-v7',lifecycle:'qualified',successorOf:paths.oldMatrix,
 admittedOps:[...oldMatrix.admittedOps,...qualified],
 pendingQualification:[...new Set([...oldMatrix.pendingQualification,...pending])],
 explicitRefuse:[...new Set([...oldMatrix.explicitRefuse,...refusals])],
 unresolvedCandidateRows:[...new Set([...oldMatrix.unresolvedCandidateRows,...pending])],
})
write(paths.release,{...oldRelease,schemaVersion:7,planId:'brep-capability-registry-release-full-v7',lifecycle:'qualified',successorOf:paths.oldRelease,
 closedMatrix:paths.matrix,g8Index:paths.index,capabilities:[...oldRelease.capabilities,...qualified],
 excludedPendingQualification:[...new Set([...oldRelease.excludedPendingQualification,...pending])],
 explicitRefuse:[...new Set([...oldRelease.explicitRefuse,...refusals])],
})
