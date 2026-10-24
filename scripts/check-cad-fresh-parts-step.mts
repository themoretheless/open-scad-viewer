/** Fresh native construction/edit/fillet/export chain; browser acceptance is separate. */
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdir,readFile,writeFile} from 'node:fs/promises'
import {resolve} from 'node:path'
import identity from '../src/generated/geometry-kernels/identity'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {cadRoadmapParts} from '../benchmarks/cad-roadmap-fixtures'
import {transformNurbsBrep,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {solidExactEdgeFeature} from '../src/services/solidExactEdgeFeature'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
import {DirectHistory,parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'

assert(process.argv[2],'Provide a new qualification output directory')
const output=resolve(process.argv[2]);await mkdir(output)
const hash=(bytes:string|Uint8Array)=>createHash('sha256').update(bytes).digest('hex')
const wasm=await readFile('public/wasm/geometry-kernel.wasm')
assert.equal(hash(wasm),identity.sha256);assert.equal(wasm.length,identity.byteLength)
await warmGeometryKernel()
const shift=[3,-2,1],parts:unknown[]=[],chains:unknown[]=[]
for(const fixture of cadRoadmapParts().filter(p=>['bracket','flange','enclosure'].includes(p.body.id))){
 const source=fixture.body,rawSourceJson=serializeDirectDocument({version:1,sketches:[],bodies:[source]})
 const sourceDocument=parseDirectDocument(rawSourceJson)
 assert.deepEqual(sourceDocument.bodies[0].brep,source.brep)
 const sourceJson=serializeDirectDocument(sourceDocument)
 const history=new DirectHistory(sourceDocument)
 const placed=transformNurbsBrep(source.brep!,[[1,0,0,shift[0]],[0,1,0,shift[1]],[0,0,1,shift[2]],[0,0,0,1]])
 const edited={...source,brep:placed,mesh:tessellateNurbsBrep(placed,8)}
 history.commit({...history.document,bodies:[edited]})
 const edges=placed.edges.flatMap((edge,i)=>{
  const points=edge.vertices.map(v=>placed.vertices[v].point)
  if(source.id==='flange')return edge.curve.degree===2&&points.every(p=>Math.abs(p[2]-7)<1e-8&&Math.abs(Math.hypot(p[0]-3,p[1]+2)-20)<1e-8)?[i]:[]
  return points.every(p=>p[0]===3&&p[1]===-2)?[i]:[]
 })
 assert(edges.length>0)
 const rounded=solidExactEdgeFeature(edited,edges,1,'fillet','brep')
 assert(rounded.evidence.audit.ok&&rounded.evidence.namingComplete&&rounded.evidence.certificate.complete)
 history.commit({...history.document,bodies:[rounded.body]})
 const finalJson=serializeDirectDocument(history.document)
 assert.deepEqual(history.undo().bodies[0].brep,placed)
 assert.equal(serializeDirectDocument(history.undo()),sourceJson)
 history.redo();assert.equal(serializeDirectDocument(history.redo()),finalJson)
 const restored=parseDirectDocument(finalJson)
 assert.equal(serializeDirectDocument(restored),finalJson)
 assert.equal(restored.bodies[0].id,source.id)
 assert.deepEqual(restored.bodies[0].brep!.topologyIds,rounded.body.brep!.topologyIds)
 const removed=source.id==='flange'?2*Math.PI*(19*(1-Math.PI/4)+1/6):20*(1-Math.PI/4)
 for(const [stage,body,bounds,volume] of [
  ['source',source,fixture.bounds,fixture.volume],
  ['edited-rounded',restored.bodies[0],fixture.bounds.map(p=>p.map((v,i)=>v+shift[i])),fixture.volume-removed],
 ] as const){
  const file=source.id+'-'+stage+'.step',text=await exportSolidStepCurrent(body)
  await writeFile(resolve(output,file),text,{flag:'wx'})
  parts.push({name:source.id+'-'+stage,file,sha256:hash(text),expected:{boundsMm:bounds,volumeMm3:volume,solids:1}})
 }
 await writeFile(resolve(output,source.id+'-restored.json'),finalJson,{flag:'wx'})
 chains.push({part:source.id,operations:[source.id==='enclosure'?'box Boolean difference':'profile extrusion','translate','exact fillet','STEP export','Undo twice','Redo twice','JSON restore'],selectedBrepEdges:edges,capability:rounded.evidence.certificate.capability,identityPreserved:true,historyRoundTripExact:true,documentRoundTripExact:true})
 assert.equal(serializeDirectDocument({version:1,sketches:[],bodies:[source]}),rawSourceJson)
}
assert.equal(chains.length,3)
assert.equal(hash(await readFile('public/wasm/geometry-kernel.wasm')),identity.sha256)
await writeFile(resolve(output,'manifest.json'),JSON.stringify({schema:'fresh-cad-parts-step/1',wasmSha256:identity.sha256,toleranceMm:1e-5,relativeVolumeTolerance:1e-7,parts,chains,independentStepQualified:false,browserQualified:false,scope:'Fresh native controlled-class chains, two commits per part; JSON restore is not browser reload. Independent STEP checks required.'},null,2)+'\n',{flag:'wx'})
console.log(JSON.stringify({passed:true,parts:chains.length,stepFiles:parts.length,wasmSha256:identity.sha256}))
