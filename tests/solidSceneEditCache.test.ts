import {expect,it,vi} from 'vitest'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {parseDirectDocument,serializeDirectDocument,type DirectDocument} from '../src/services/directModeling'
import {applySolidSceneEdit} from '../src/services/solidSceneEdit'
import {SolidInstanceBatchCache} from '../src/services/solidInstanceBatchCache'
it('reuses transformed instance batches during authoritative result validation',async()=>{
 await warmGeometryKernel()
 const brep=createBrepBox([0,0,0],[2,3,4]),mesh=tessellateNurbsBrep(brep,1)
 const document:DirectDocument={version:1,sketches:[],bodies:[{id:'source',name:'Source',brep,mesh},{id:'linked',name:'Linked',brep,mesh,instance:{sourceId:'source',matrix:[[1,0,0,10],[0,1,0,0],[0,0,1,0],[0,0,0,1]]}}]}
 const before=structuredClone(document),cache=new SolidInstanceBatchCache(),get=vi.spyOn(cache,'get')
 const result=applySolidSceneEdit(document,{operation:'transform',id:'source',ids:['source'],createdId:'',x:5,y:0,z:0,axis:'z',angle:0,scale:1},cache)
 expect(cache.size).toBeGreaterThan(0);get.mockClear()
 const validated=parseDirectDocument(serializeDirectDocument(result),cache)
 expect(get.mock.results.some(row=>row.type==='return'&&row.value!==undefined)).toBe(true)
 expect(Array.from(validated.bodies[1].mesh.positions)).toEqual(Array.from(result.bodies[1].mesh.positions))
 expect(Math.min(...Array.from(validated.bodies[1].mesh.positions).filter((_,i)=>i%3===0))).toBe(15)
 expect(document).toEqual(before)
})

it('reuses exact cached batches for instance creation and placement without sharing caller geometry',async()=>{
 await warmGeometryKernel()
 const brep=createBrepBox([0,0,0],[2,3,4]),mesh=tessellateNurbsBrep(brep,1)
 const document:DirectDocument={version:1,sketches:[],bodies:[{id:'source',name:'Source',brep,mesh}]}
 const before=structuredClone(document),cache=new SolidInstanceBatchCache(),get=vi.spyOn(cache,'get')
 const options={operation:'instance-create' as const,id:'source',createdId:'linked',x:10,y:0,z:0,axis:'z' as const,angle:0,scale:1}
 const created=applySolidSceneEdit(document,options,cache)
 expect(created).toEqual(applySolidSceneEdit(document,options))
 expect(cache.size).toBeGreaterThan(0)
 get.mockClear()
 const placed=applySolidSceneEdit(created,{...options,operation:'instance-transform',id:'linked',x:2},cache)
 expect(get.mock.results.some(row=>row.type==='return'&&row.value!==undefined)).toBe(true)
 expect(placed).toEqual(applySolidSceneEdit(created,{...options,operation:'instance-transform',id:'linked',x:2}))
 expect(document).toEqual(before)
 const original=structuredClone(created)
 placed.bodies[0].mesh.positions[0]=999
 expect(created).toEqual(original)
})
