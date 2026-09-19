import {readFileSync} from 'node:fs'
import {IDBFactory} from 'fake-indexeddb'
import {afterEach, beforeEach, expect, it, vi} from 'vitest'
import {emptyDirectDocument} from '../src/services/directModeling'
import {prepareSolidStepImport, exportSolidStepOriginal} from '../src/services/solidStepExchange'
import * as store from '../src/services/cadStepIndexedDb'

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
