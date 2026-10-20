import {readFileSync,writeFileSync} from 'node:fs'
import {join} from 'node:path'
import assert from 'node:assert/strict'
import {applySolidMinkowski} from '../src/services/solidMinkowski'
import {verifyMinkowskiProof} from '../src/services/geometry/minkowski3d'
import {normalizePolygonMesh} from '../src/services/geometry/polygon'
import {mainSolidResult,mainSolidExpectation} from '../src/services/mainSolidProtocol'
import artifactIdentity from '../src/generated/geometry-kernels/identity'
import {box3d} from '../tests/fixtures/minkowski3d-cases'
const results=[]
for(const name of ['curved-through-hole','sealed-curved-cavity','sheared-through-hole','sheared-nonstar','sheared-sealed-cavity']){
 const brep=JSON.parse(readFileSync(join(process.argv[2],name+'.json'),'utf8'))
 // Viewport data is deliberately independent: source authority is the bound
 // original B-rep, not the display mesh used to select the body.
 const document={version:1 as const,sketches:[],bodies:[{id:'a',name,brep,mesh:normalizePolygonMesh(box3d([0,0,0],[4,4,4]))},{id:'b',name:'kernel',mesh:normalizePolygonMesh(box3d([0,0,0],[.25,.25,.25]))}]},before=structuredClone(document)
 const options={ids:['a','b'] as [string,string],createdId:'sum',ru:true,maximumSourceDeviationMm:100}
 const r=applySolidMinkowski(document,options)
 assert.deepEqual(document,before);assert.equal(r.document.bodies.length,3);assert.deepEqual(r.sourceApproximationTopology,[true,true]);assert(mainSolidResult(mainSolidExpectation({kind:'minkowski3d',document,options}),r));assert.deepEqual(verifyMinkowskiProof(JSON.parse(JSON.stringify(r.proof))).mesh,r.document.bodies[2].mesh)
 const fake=structuredClone(r);fake.sourceApproximationTopology=[false,true];assert(!mainSolidResult(mainSolidExpectation({kind:'minkowski3d',document,options}),fake))
 results.push({name,proof:r.proof,sourceApproximationTopology:r.sourceApproximationTopology,approximationTopology:r.approximationTopology,volumeMm3:r.volumeMm3,originalDocumentPreserved:true,workerProtocolAccepted:true});console.error(name,'host verified')
 writeFileSync(process.argv[3],JSON.stringify({wasmSha256:artifactIdentity.sha256,results},null,2)+'\n')
}
