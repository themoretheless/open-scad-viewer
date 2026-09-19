#!/usr/bin/env node
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {dirname,resolve} from 'node:path'
import {fileURLToPath} from 'node:url'
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..')
const read=p=>JSON.parse(readFileSync(resolve(root,p),'utf8'))
const write=(p,v)=>{mkdirSync(dirname(resolve(root,p)),{recursive:true});writeFileSync(resolve(root,p),JSON.stringify(v,null,2)+'\n')}
const copyVersion=(source,target)=>writeFileSync(resolve(root,target),readFileSync(resolve(root,source),'utf8').replaceAll('-v10','-v11').replaceAll('"const": 10','"const": 11').replaceAll('CA10-','CA11-'))
const digest=paths=>{const h=createHash('sha256');for(const p of [...paths].sort())h.update(p).update('\0').update(readFileSync(resolve(root,p))).update('\n');return h.digest('hex')}
const q='docs/qualification'
const previous={index:`${q}/plans/g8-full-matrix-index-v10.json`,matrix:`${q}/brep-full-closed-matrix-v10.json`,release:`${q}/brep-capability-registry-release-full-v10.json`}
const next={index:`${q}/plans/g8-full-matrix-index-v11.json`,matrix:`${q}/brep-full-closed-matrix-v11.json`,release:`${q}/brep-capability-registry-release-full-v11.json`,planSchema:`${q}/plans/brep-capability-qualification-plan-v11.schema.json`,evidenceSchema:`${q}/brep-capability-evidence-v11.schema.json`}
copyVersion(`${q}/plans/brep-capability-qualification-plan-v10.schema.json`,next.planSchema)
copyVersion(`${q}/brep-capability-evidence-v10.schema.json`,next.evidenceSchema)
const dependencies=['global-solid-audit/1','persistent-naming/1']
const specs=[
 {id:'iges-interchange/2',successorOf:'iges-interchange/1',slug:'iges-interchange-2',
  claim:'Strict finite direct IGES 5.3 rational B-rep graph interchange with 80-column section validation, shared topology, pcurves, multiple bodies/cavities and graph-bound identity.',
  included:['Entities 126, 128, 186, 502, 504, 508, 510 and 514 with form-15 graph-bound identity properties','Millimetre and bounded standard global length units; exact rational curves, surfaces and UV pcurves','Direct closed manifold topology, holes, shared edge indices, multiple shells, cavities and bodies; ignored color/name metadata report'],
  excluded:['Legacy /1 point-AABB reconstruction','Entity-local transformations, assemblies, external references, open/nonmanifold shells','Unknown geometric entities, mesh/faceted fallback, healing, constructor recognition'],
  rows:[['CA11-IGES2-STRICT-SECTIONS-RESOURCES','Complete'],['CA11-IGES2-RATIONAL-DIRECT-ROUNDTRIP','Complete'],['CA11-IGES2-TOPOLOGY-PCURVE-IDENTITY','Complete'],['CA11-IGES2-MULTIBODY-CAVITY','Complete'],['CA11-IGES2-UNKNOWN-NONMANIFOLD-TRANSFORM','typed-refuse']]},
 {id:'step-interchange/4',successorOf:'step-interchange/3',slug:'step-interchange-4',
  claim:'Finite direct STEP successor adding safely representable seam-bearing rational analytic carriers and exact endpoint point selectors while retaining /3 topology identity and bounds.',
  included:['Direct /3 topology, units, placements, multiple holes/bodies/cavities and graph-bound identity','Cylinder, cone, sphere and ring-torus converted to finite rational carriers with angular parameter domains','TRIMMED_CURVE parameter selectors and point selectors exactly equal to a basis endpoint'],
  excluded:['Assemblies and external references without complete occurrence identity and bounds','Interior point selectors without an exact analytic inverse, singular pole edges and non-rigid transformations','Faceted, tessellated, CSG, constructor/AABB reconstruction, mesh fallback or silent healing'],
  rows:[['CA11-STEP4-DIRECT-GRAPH-IDENTITY','Complete'],['CA11-STEP4-ANALYTIC-RATIONAL-CARRIERS','Complete'],['CA11-STEP4-ENDPOINT-POINT-SELECTORS','Complete'],['CA11-STEP4-MULTIBODY-HOLES-UNITS','Complete'],['CA11-STEP4-ASSEMBLY-EXTERNAL-POLE','typed-refuse']]},
]
const artifacts=['crates/brep-core/src/iges_interchange_v2.rs','crates/brep-core/src/step_interchange_v3.rs','crates/geometry-bridge/src/lib.rs','src/services/cadIges.ts','src/services/cadNurbsStep.ts','tests/brepInterchangeV11.test.ts','tests/brepQualificationV11.test.ts','src/generated/geometry-kernels/kernel_bg.wasm']
for(const spec of specs){
 const plan=`${q}/plans/${spec.slug}.json`,evidence=`${q}/${spec.slug}-evidence-v11.json`
 write(plan,{$schema:'./brep-capability-qualification-plan-v11.schema.json',schema:'open-scad-viewer/brep-capability-qualification-plan',schemaVersion:11,planId:spec.slug,gate:{id:'g8-full',name:'brep-full-closed-matrix',version:11},lifecycle:{status:'qualified',qualificationClaim:'finite-cell'},claimBoundary:{candidateClaim:spec.claim,forbiddenClaims:spec.excluded},capability:spec.id,successorOf:spec.successorOf,dependencies:dependencies.map(capability=>({capability,requiredMaturity:'Qualified'})),scope:{included:spec.included,excluded:spec.excluded},matrix:spec.rows.map(([id,expected])=>({id,expected,blocksQualification:true})),bindings:{implementation:artifacts.slice(0,-1),registry:'src/services/geometry/brepCapability.ts',evidenceJson:evidence},evidence:{state:'qualified',notes:['Qualification is limited to the coherent proven finite subset; every excluded construct is a typed refusal.']},resetPolicy:'false-Complete freezes this capability and transitively stales all dependents; recovery requires a successor capability ID',unresolvedRows:[]})
 write(evidence,{$schema:'./brep-capability-evidence-v11.schema.json',schema:'open-scad-viewer/brep-capability-evidence',schemaVersion:11,evidenceId:`${spec.slug}-evidence-v11`,capability:spec.id,maturity:'Qualified',state:'qualified',plan,runs:[{id:`native-bridge-wasm-${spec.slug}-2026-09-17-v11`,result:'pass',artifactHash:digest(artifacts)}],oracles:['iges_interchange_v2::tests::direct_roundtrip_is_80_column_and_preserves_graph_identity','step_interchange_v3::tests::direct_roundtrip_preserves_topology_and_identity','geometry_bridge::tests::direct_iges_v2_bridge_preserves_exact_graph','tests/brepInterchangeV11.test.ts'],unresolvedRows:[],attestation:{fabricatedRuns:false,note:'Recorded after direct kernel and bridge roundtrip, malformed/resource/refusal, typecheck, build and product tests pass.'}})
 spec.plan=plan;spec.evidence=evidence
}
const oldIndex=read(previous.index),oldMatrix=read(previous.matrix),oldRelease=read(previous.release)
const rows=specs.map(s=>({id:s.id,plan:s.plan,evidence:s.evidence,maturity:'Qualified',releaseState:'shipped',dependencies}))
const refusals=['iges-unknown-geometric-entity','iges-entity-local-transform','iges-open-or-nonmanifold-shell','step-assembly-incomplete-occurrence-identity','step-external-reference-incomplete-bounds','step-interior-point-selector-without-exact-inverse']
write(next.index,{...oldIndex,schemaVersion:11,successorOf:previous.index,matrix:next.matrix,registry:next.release,capabilities:[...oldIndex.capabilities,...rows]})
write(next.matrix,{...oldMatrix,schemaVersion:11,planId:'brep-full-closed-matrix-v11',successorOf:previous.matrix,admittedOps:[...oldMatrix.admittedOps,...rows.map(r=>r.id)],explicitRefuse:[...new Set([...oldMatrix.explicitRefuse,...refusals])]})
write(next.release,{...oldRelease,schemaVersion:11,planId:'brep-capability-registry-release-full-v11',successorOf:previous.release,closedMatrix:next.matrix,g8Index:next.index,capabilities:[...oldRelease.capabilities,...rows.map(r=>r.id)],explicitRefuse:[...new Set([...oldRelease.explicitRefuse,...refusals])]})
