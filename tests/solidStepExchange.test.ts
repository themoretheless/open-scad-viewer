import {readFileSync} from 'node:fs'
import {IDBFactory} from 'fake-indexeddb'
import {afterEach, beforeEach, expect, it, vi} from 'vitest'
import {emptyDirectDocument} from '../src/services/directModeling'
import {prepareSolidStepImport, exportSolidStepOriginal, exportSolidStepCurrent} from '../src/services/solidStepExchange'
import * as store from '../src/services/cadStepIndexedDb'
import * as kernel from '../src/services/geometry/kernel'

const source=readFileSync(new URL('./fixtures/step-v6/self-authored-ap242-assembly.step',import.meta.url),'utf8')
beforeEach(()=>vi.stubGlobal('indexedDB',new IDBFactory()))
afterEach(()=>{vi.restoreAllMocks();vi.unstubAllGlobals()})

it('prepares independent editable bodies while retaining the exact original graph',async()=>{
 const before=emptyDirectDocument(), copy=structuredClone(before)
 const imported=await prepareSolidStepImport(source,before)
 expect(before).toEqual(copy)
 expect(imported.document.bodies).toHaveLength(1)
 expect(imported.document.bodies[0]!.brep!.bodies).toHaveLength(3)
 expect(imported.report.occurrenceIdentities).toHaveLength(3)
 expect(imported.document.interchange?.step?.retained).toBe(true)
 imported.document.bodies[0]!.mesh.positions[0]=999
 expect(await exportSolidStepOriginal()).toBe(source)
})

it('checks whole-scene capacity before replacing a saved original',async()=>{
 const imported=await prepareSolidStepImport(source,emptyDirectDocument())
 const full={...emptyDirectDocument(),bodies:Array.from({length:200},(_,i)=>({...imported.document.bodies[0]!,id:String(i)}))}
 const save=vi.spyOn(store,'saveProjectStepModel')
 await expect(prepareSolidStepImport(source,full)).rejects.toThrow('Invalid direct modeling document')
 expect(save).not.toHaveBeenCalled()
 expect(await exportSolidStepOriginal()).toBe(source)
})

it('does not return an addition if durable persistence fails',async()=>{
 const before=emptyDirectDocument()
 vi.spyOn(store,'saveProjectStepModel').mockRejectedValue(new Error('quota exceeded'))
 await expect(prepareSolidStepImport(source,before)).rejects.toThrow('quota exceeded')
 expect(before).toEqual(emptyDirectDocument())
 expect(await store.loadProjectStepModel()).toBeUndefined()
})

it('refuses an absent original and preserves the previous one after a bad import',async()=>{
 await expect(exportSolidStepOriginal()).rejects.toThrow('No saved AP242 original')
 await prepareSolidStepImport(source,emptyDirectDocument())
 await expect(prepareSolidStepImport('not STEP',emptyDirectDocument())).rejects.toThrow()
 expect(await exportSolidStepOriginal()).toBe(source)
})

it('waits for kernel readiness and snapshots the import scene before yielding',async()=>{
 await kernel.warmGeometryKernel()
 let release!:()=>void
 const pending=new Promise<void>(resolve=>{release=resolve})
 vi.spyOn(kernel,'warmGeometryKernel').mockReturnValue(pending)
 const save=vi.spyOn(store,'saveProjectStepModel')
 const before=emptyDirectDocument()
 const importing=prepareSolidStepImport(source,before)
 before.sketches.push({id:'later',name:'Later',closed:true,points:[[0,0],[1,0],[0,1]]})
 await Promise.resolve()
 expect(save).not.toHaveBeenCalled()
 release()
 const imported=await importing
 expect(imported.document.sketches).toHaveLength(0)
 expect(before.sketches).toHaveLength(1)
 expect(save).toHaveBeenCalledOnce()
})

it('does not access retained native data before warm-up and recovers after failure',async()=>{
 await prepareSolidStepImport(source,emptyDirectDocument())
 const load=vi.spyOn(store,'loadProjectStepModel')
 const warm=vi.spyOn(kernel,'warmGeometryKernel').mockRejectedValueOnce(Error('warm failed'))
 await expect(exportSolidStepOriginal()).rejects.toThrow('warm failed')
 expect(load).not.toHaveBeenCalled()
 let release!:()=>void
 warm.mockReturnValueOnce(new Promise<void>(resolve=>{release=resolve}))
 const exporting=exportSolidStepOriginal()
 await Promise.resolve()
 expect(load).not.toHaveBeenCalled()
 release()
 expect(await exporting).toBe(source)
 expect(load).toHaveBeenCalledOnce()
})

it('exports current edited B-rep geometry rather than its retained STEP source',async()=>{
 const {createBrepBox,transformNurbsBrep,tessellateNurbsBrep}=await import('../src/services/geometry/brep')
 const {importDirectStepV9}=await import('../src/services/cadNurbsStep')
 const brep=transformNurbsBrep(createBrepBox([0,0,0],[2,3,4]),[[2,0,0,5],[0,1,0,0],[0,0,1,0],[0,0,0,1]])
 const mesh=tessellateNurbsBrep(brep)
 const text=await exportSolidStepCurrent({id:'edited',name:'Edited',brep,mesh})
 const imported=importDirectStepV9(text)
 const xs=imported.model.vertices.map(v=>v.point[0])
 expect(Math.min(...xs)).toBeCloseTo(5);expect(Math.max(...xs)).toBeCloseTo(9)
 await expect(exportSolidStepCurrent({id:'mesh',name:'Mesh',mesh})).rejects.toThrow('exact B-rep')
})
