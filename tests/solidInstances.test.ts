import {SolidInstanceBatchCache} from '../src/services/solidInstanceBatchCache'
import {transformPolygonMesh} from '../src/services/geometry/polygon'
import {it,expect} from 'vitest'
import {createSolidInstance,detachSolidInstances,resolveSolidInstances,transformSolidInstance} from '../src/services/solidInstances'
import {emptyDirectDocument,DirectHistory,parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {createBrepBox,tessellateNurbsBrep,transformNurbsBrep} from '../src/services/geometry/brep'
import {stringifyMeshJson} from '../src/services/meshJson'
import {transformSelection,transformBodies,pushPullFace,splitSolid} from '../src/services/directSolidTools'
const placement=[[1,0,0,10],[0,1,0,0],[0,0,1,0],[0,0,0,1]]
const box=(size:number)=>{const brep=createBrepBox([0,0,0],[size,2,3]);return {id:'source',name:'Source',brep,mesh:tessellateNurbsBrep(brep,1)}}
const maxX=(positions:ArrayLike<number>)=>Math.max(...Array.from(positions).filter((_,i)=>i%3===0))
it('rotates and scales an instance about its center while retaining source identity and history',()=>{
 const seed=emptyDirectDocument();seed.bodies.push(box(1))
 const linked=createSolidInstance(seed,'source','instance',placement),history=new DirectHistory(linked)
 const transformed=transformSolidInstance(linked,'instance',[1,0,0],[0,0,1],90,2)
 expect(transformed.bodies[1].instance!.sourceId).toBe('source')
 expect(maxX(transformed.bodies[1].mesh.positions)).toBeCloseTo(13.5,10)
 expect(stringifyMeshJson(transformed.bodies[0])).toBe(stringifyMeshJson(linked.bodies[0]))
 history.commit(transformed)
 expect(maxX(history.undo().bodies[1].mesh.positions)).toBe(11)
 expect(maxX(history.redo().bodies[1].mesh.positions)).toBeCloseTo(13.5,10)
 const edited=history.document;edited.bodies[0]=box(4);history.commit(edited)
 const restored=parseDirectDocument(stringifyMeshJson(history.document)),ys=Array.from(restored.bodies[1].mesh.positions).filter((_,i)=>i%3===1)
 expect(Math.max(...ys)-Math.min(...ys)).toBeCloseTo(8,10)
 expect(()=>transformSolidInstance(linked,'instance',[0,0,0],[0,0,1],0,0)).toThrow()
})
it('rejects geometry edits of linked bodies before changing their cached shape',()=>{
 const seed=emptyDirectDocument();seed.bodies.push(box(1))
 const linked=createSolidInstance(seed,'source','instance',placement),before=stringifyMeshJson(linked),body=linked.bodies[1]
 expect(()=>transformBodies([body],[1,0,0],[0,0,1],0,1)).toThrow('detach the instance')
 expect(()=>pushPullFace(body,0,1)).toThrow('detach the instance')
 expect(()=>splitSolid(body,[1,0,0],10.5)).toThrow('detach the instance')
 expect(stringifyMeshJson(linked)).toBe(before)
})
it('updates linked geometry from its source across undo, redo and reload',()=>{
 const seed=emptyDirectDocument();seed.bodies.push(box(1));const before=stringifyMeshJson(seed)
 const linked=createSolidInstance(seed,'source','instance',placement)
 expect(maxX(linked.bodies[1].mesh.positions)).toBe(11);expect(stringifyMeshJson(seed)).toBe(before)
 const history=new DirectHistory(linked),edit=history.document;edit.bodies[0]=box(4);history.commit(edit)
 expect(maxX(history.document.bodies[1].mesh.positions)).toBe(14)
 expect(history.document.bodies[1].id).toBe('instance')
 expect(maxX(history.undo().bodies[1].mesh.positions)).toBe(11)
 expect(maxX(history.redo().bodies[1].mesh.positions)).toBe(14)
 expect(maxX(parseDirectDocument(stringifyMeshJson(history.document)).bodies[1].mesh.positions)).toBe(14)
})
it('detaches the current geometry and refuses missing or chained sources',()=>{
 const seed=emptyDirectDocument();seed.bodies.push(box(1));const linked=createSolidInstance(seed,'source','instance',placement)
 const independent=detachSolidInstances(linked,['instance']);independent.bodies[0]=box(4)
 expect(maxX(resolveSolidInstances(independent).bodies[1].mesh.positions)).toBe(11)
 const missing=structuredClone(linked);missing.bodies.shift();expect(()=>resolveSolidInstances(missing)).toThrow('source is missing')
 expect(()=>createSolidInstance(linked,'instance','nested',placement)).toThrow('independent')
 const chain=structuredClone(linked);chain.bodies[0].instance={sourceId:'instance',matrix:placement}
 expect(()=>resolveSolidInstances(chain)).toThrow('independent')
})
it('rejects invalid placement and duplicate identity',()=>{
 const seed=emptyDirectDocument();seed.bodies.push(box(1))
 expect(()=>createSolidInstance(seed,'source','source',placement)).toThrow('unique')
 expect(()=>createSolidInstance(seed,'source','bad',[[1]])).toThrow('affine')
 const invalid=structuredClone(placement);invalid[0][0]=NaN
 expect(()=>createSolidInstance(seed,'source','bad',invalid)).toThrow('finite')
})


it('stores linked geometry once and restores independent instance metadata',()=>{
 const seed=emptyDirectDocument();seed.bodies.push(box(1))
 const linked=createSolidInstance(seed,'source','instance',placement)
 linked.bodies[1].material={name:'Red',color:'#ff0000'}
 const full=stringifyMeshJson(linked),compact=serializeDirectDocument(linked),wire=JSON.parse(compact)
 expect(wire.bodies[1].mesh).toBeUndefined();expect(wire.bodies[1].brep).toBeUndefined()
 expect(compact.length).toBeLessThan(full.length*.65)
 const restored=parseDirectDocument(compact)
 expect(stringifyMeshJson(restored)).toBe(stringifyMeshJson(parseDirectDocument(full)))
 expect(restored.bodies[1].material).toEqual({name:'Red',color:'#ff0000'})
 wire.bodies[1].brep={};expect(()=>parseDirectDocument(JSON.stringify(wire))).toThrow('mesh')
 delete wire.bodies[1].brep;wire.bodies[1].instance.sourceId='missing'
 expect(()=>parseDirectDocument(JSON.stringify(wire))).toThrow('source is missing')
})

it('bounds compact instance expansion before materializing repeated source geometry',()=>{
 const seed=emptyDirectDocument();seed.bodies.push(box(1))
 const linked=createSolidInstance(seed,'source','instance',placement),wire=JSON.parse(serializeDirectDocument(linked))
 wire.bodies[0].extension='x'.repeat(1_000_000)
 for(let i=0;i<70;i++)wire.bodies.push({...wire.bodies[1],id:`instance-${i}`})
 const compact=JSON.stringify(wire)
 expect(compact.length).toBeLessThan(2_000_000)
 expect(()=>parseDirectDocument(compact)).toThrow('Expanded instance document exceeds 64 MB')
})

it('retains compact history for source edits without owning geometry in inactive states',()=>{
 const seed=emptyDirectDocument();seed.bodies.push(box(1))
 let linked=seed
 for(let i=0;i<20;i++)linked=createSolidInstance(linked,'source',`instance-${i}`,placement)
 const history=new DirectHistory(linked),fullCharacters=stringifyMeshJson(history.document).length
 for(let width=2;width<=6;width++){const edit=history.document;edit.bodies[0]=box(width);history.commit(edit)}
 expect(history.storageStats).toMatchObject({undoStates:5,redoStates:0,materializedStates:1})
 expect(history.storageStats.retainedCharacters).toBeLessThan(fullCharacters)
 for(let width=5;width>=1;width--)expect(maxX(history.undo().bodies[20].mesh.positions)).toBe(10+width)
 for(let width=2;width<=6;width++)expect(maxX(history.redo().bodies[20].mesh.positions)).toBe(10+width)
 const borrowed=history.document;borrowed.bodies[0].name='mutated'
 expect(history.document.bodies[0].name).toBe('Source')
 expect(history.storageStats.materializedStates).toBe(1)
})

it('admits 1000 bounded linked instances while retaining independent-object and expansion limits',()=>{
 const seed=emptyDirectDocument();seed.bodies.push(box(1));delete seed.bodies[0].brep
 const wire=JSON.parse(serializeDirectDocument(createSolidInstance(seed,'source','instance',placement)))
 wire.bodies=[wire.bodies[0],...Array.from({length:1000},(_,i)=>({...wire.bodies[1],id:`instance-${i}`}))]
 const restored=parseDirectDocument(JSON.stringify(wire))
 expect(restored.bodies).toHaveLength(1001)
 expect(maxX(restored.bodies[1000].mesh.positions)).toBe(11)
 wire.bodies.push({...wire.bodies[1],id:'overflow'})
 expect(()=>parseDirectDocument(JSON.stringify(wire))).toThrow('1000 linked instances')
 const independent=JSON.parse(stringifyMeshJson(seed))
 independent.bodies=Array.from({length:201},(_,i)=>({...independent.bodies[0],id:`source-${i}`}))
 expect(()=>parseDirectDocument(JSON.stringify(independent))).toThrow('200 independent')
})


it('batches interleaved source placements with the same mesh and B-rep as individual transforms',()=>{
 const sourceA=box(1),sourceB={...box(3),id:'source-b',name:'Source B'}
 const reflection=[[-1,0,0,10],[0,1,0,0],[0,0,1,0],[0,0,0,1]]
 const doc={...emptyDirectDocument(),bodies:[sourceA,
  {...structuredClone(sourceB),id:'b-instance',instance:{sourceId:sourceB.id,matrix:placement}},sourceB,
  {...structuredClone(sourceA),id:'a-instance',instance:{sourceId:sourceA.id,matrix:reflection}},
  {...structuredClone(sourceA),id:'a-instance-2',instance:{sourceId:sourceA.id,matrix:placement}}]}
 const before=stringifyMeshJson(doc),resolved=resolveSolidInstances(doc)
 expect(resolved.bodies.map(body=>body.id)).toEqual(doc.bodies.map(body=>body.id))
 for(const body of resolved.bodies.filter(body=>body.instance)){
  const source=body.instance!.sourceId===sourceA.id?sourceA:sourceB,matrix=body.instance!.matrix
  const individual=transformPolygonMesh(source.mesh,matrix)
  expect(Array.from(body.mesh.positions)).toEqual(Array.from(individual.positions))
  expect(Array.from(body.mesh.indices)).toEqual(Array.from(individual.indices))
  expect(body.brep).toEqual(transformNurbsBrep(source.brep!,matrix))
 }
 expect(stringifyMeshJson(doc)).toBe(before)
})


it.each([
 {ids:['instance'],delta:[2,3,1],angle:0,scale:1},
 {ids:['source','instance'],delta:[2,3,1],angle:0,scale:1},
 {ids:['source','instance'],delta:[2,3,1],angle:90,scale:2},
 {ids:['instance','second'],delta:[2,3,1],angle:35,scale:.5},
])('transforms linked selection exactly once: $ids / $angle degrees',({ids,delta,angle,scale})=>{
 const seed=emptyDirectDocument();seed.bodies.push(box(1))
 const first=createSolidInstance(seed,'source','instance',placement)
 const matrix=structuredClone(placement);matrix[1][3]=5
 const linked=createSolidInstance(first,'source','second',matrix)
 const independent=detachSolidInstances(linked,['instance','second'])
 const expected=transformSelection(independent,ids,delta as [number,number,number],[0,0,1],angle,scale)
 const transformed=transformSelection(linked,ids,delta as [number,number,number],[0,0,1],angle,scale)
 const history=new DirectHistory(linked);history.commit(transformed)
 for(const id of ids){
  const actual=history.document.bodies.find(body=>body.id===id)!,reference=expected.bodies.find(body=>body.id===id)!
  expect(Array.from(actual.mesh.positions)).toHaveLength(reference.mesh.positions.length)
  Array.from(actual.mesh.positions).forEach((value,i)=>expect(value).toBeCloseTo(reference.mesh.positions[i],9))
  if(id!=='source')expect(actual.instance!.sourceId).toBe('source')
 }
 expect(stringifyMeshJson(history.undo())).toBe(stringifyMeshJson(new DirectHistory(linked).document))
 const redone=history.redo();expect(redone.bodies[1].instance).toBeDefined()
 expect(stringifyMeshJson(parseDirectDocument(serializeDirectDocument(redone)))).toBe(stringifyMeshJson(redone))
})

it('checks resolved linked geometry before changing history and isolates policy mutations',()=>{
 const seed=emptyDirectDocument();seed.bodies.push(box(1))
 const linked=createSolidInstance(seed,'source','instance',placement),history=new DirectHistory(linked)
 const edit=history.document;edit.bodies[0]=box(4)
 const before=stringifyMeshJson(history.document),stats=history.storageStats
 expect(()=>history.commit(edit,resolved=>{
  expect(maxX(resolved.bodies[1].mesh.positions)).toBe(14)
  throw Error('locked linked body')
 })).toThrow('locked linked body')
 expect(stringifyMeshJson(history.document)).toBe(before);expect(history.storageStats).toEqual(stats)
 history.commit(edit,resolved=>{resolved.bodies[1].mesh.positions.fill(0);resolved.bodies[0].name='policy mutation'})
 expect(maxX(history.document.bodies[1].mesh.positions)).toBe(14)
 expect(history.document.bodies[0].name).toBe('Source')
 expect(maxX(history.undo().bodies[1].mesh.positions)).toBe(11)
 expect(maxX(history.redo().bodies[1].mesh.positions)).toBe(14)
})

it('reuses exact instance batches without sharing mutable geometry or stale placements',()=>{
 const seed=emptyDirectDocument(),source=box(1);seed.bodies.push(source,{...source,id:'copy',instance:{sourceId:'source',matrix:structuredClone(placement)}})
 const cache=new SolidInstanceBatchCache(),first=resolveSolidInstances(seed,cache)
 expect(cache.size).toBe(1);const bytes=cache.retainedBytes
 first.bodies[1].mesh.positions[0]=999;first.bodies[1].brep!.vertices[0].point[0]=999
 const second=resolveSolidInstances(seed,cache)
 expect(cache.size).toBe(1);expect(cache.retainedBytes).toBe(bytes)
 expect(maxX(second.bodies[1].mesh.positions)).toBe(11)
 expect(second.bodies[1].brep!.vertices[0].point[0]).not.toBe(999)
 seed.bodies[0]=box(4)
 expect(maxX(resolveSolidInstances(seed,cache).bodies[1].mesh.positions)).toBe(14);expect(cache.size).toBe(2)
 seed.bodies[1].instance!.matrix[0][3]=20
 expect(maxX(resolveSolidInstances(seed,cache).bodies[1].mesh.positions)).toBe(24);expect(cache.size).toBe(3)
 const probe=new SolidInstanceBatchCache();probe.set('a',[source])
 const bounded=new SolidInstanceBatchCache(probe.retainedBytes);bounded.set('a',[source]);bounded.set('b',[source])
 expect(bounded.get('a')).toBeUndefined();expect(bounded.get('b')).toBeDefined();expect(bounded.retainedBytes).toBe(probe.retainedBytes)
 const disabled=new SolidInstanceBatchCache(0);disabled.set('a',[source]);expect(disabled.size).toBe(0)
})

it('reads all instance identities after async undo without expanding retained geometry',async()=>{
 const seed=emptyDirectDocument();seed.bodies.push(box(1))
 const linked=createSolidInstance(seed,'source','instance',placement),history=new DirectHistory(linked)
 history.commit(emptyDirectDocument())
 expect(await history.restoreAsync('undo',async text=>parseDirectDocument(text))).toBe(true)
 expect(history.storageStats.materializedStates).toBe(0)
 expect(history.objectIds).toEqual(['source','instance'])
 expect(history.storageStats.materializedStates).toBe(0)
 const restored=history.document
 expect(maxX(restored.bodies[1].mesh.positions)).toBe(11)
 expect(restored.bodies[1].brep).toEqual(linked.bodies[1].brep)
})
