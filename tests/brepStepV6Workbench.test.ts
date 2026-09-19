import {readFileSync} from 'node:fs'
import {IDBFactory} from 'fake-indexeddb'
import {describe,expect,it} from 'vitest'
import {importDirectStepV6,exportDirectStepV6} from '../src/services/cadNurbsStep'
import {MAX_RETAINED_STEP_STORE_BYTES,importStepForWorkbench,retainedStepSession,retainedStepStoreAcceptsEncodedBytes} from '../src/services/cadStepRouting'
import {cadOperation} from '../src/services/cadWorkbench'
import {parseDirectDocument} from '../src/services/directModeling'
import {loadProjectStepModel,saveProjectStepModel} from '../src/services/cadStepIndexedDb'

const values=new Map<string,string>()
Object.defineProperty(globalThis,'localStorage',{configurable:true,value:{
  getItem:(key:string)=>values.get(key)??null,setItem:(key:string,value:string)=>values.set(key,value),
  removeItem:(key:string)=>values.delete(key),key:(index:number)=>[...values.keys()][index]??null,get length(){return values.size},
}})
const fixture=(name:string)=>readFileSync(new URL(`./fixtures/step-v6/${name}`,import.meta.url),'utf8')

describe('STEP /6 retained product closure',()=>{
  it.each([
    ['self-authored-ap242-periodic-cylinder.step',true],
    ['self-authored-ap242-periodic-cone.step',true],
    ['self-authored-ap242-periodic-torus.step',true],
    ['self-authored-ap242-pole-sphere.step',true],
    ['self-authored-ap242-pole-cone.step',true],
  ])('imports, displays and reexports %s',(name,seam)=>{
    const text=fixture(name)
    expect(text.includes('SEAM_CURVE(')).toBe(seam)
    const imported=importDirectStepV6(text)
    expect(imported.model.bodies.length).toBeGreaterThan(0)
    const routed=importStepForWorkbench(text)
    expect(routed.retainedModel).toBeDefined()
    expect(routed.report.occurrenceIdentities.length).toBeGreaterThan(0)
    expect(importDirectStepV6(exportDirectStepV6(imported.model).text).model.bodies.length).toBeGreaterThan(0)
  })

  it('preserves interchange metadata through an actual document edit',()=>{
    const routed=importStepForWorkbench(fixture('self-authored-ap242-periodic-cylinder.step'))
    const body=routed.bodies[0]
    const document=parseDirectDocument(JSON.stringify({version:1,sketches:[],bodies:[body],interchange:{step:routed.report}}))
    const edited=cadOperation(document,{action:'resize',ids:[body.id],sketches:[],axis:[0,0,1],origin:[0,0,0],
      amount:0,count:1,width:5,height:6,depth:7,pitch:1,secondary:1,mode:'min',pathId:'',profileIds:[]})
    expect(edited.interchange?.step?.occurrenceIdentities).toEqual(routed.report.occurrenceIdentities)
    retainedStepSession.set(edited.bodies[0].brep,routed.report)
    expect(importDirectStepV6(exportDirectStepV6(retainedStepSession.get()!).text).model.bodies).toHaveLength(1)
  })

  it('distinguishes one shared definition from three occurrences',()=>{
    const imported=importDirectStepV6(fixture('self-authored-ap242-assembly.step'))
    expect(imported.model.bodies).toHaveLength(3)
    expect(imported.definitionIdentities).toHaveLength(1)
    expect(imported.occurrenceIdentities).toHaveLength(3)
    expect(imported.productHierarchy.filter(row=>row.startsWith('assembly-use:'))).toHaveLength(2)
  })

  it('uses exact UTF-8 bytes at the retained-store boundary',()=>{
    expect(retainedStepStoreAcceptsEncodedBytes('é'.repeat(MAX_RETAINED_STEP_STORE_BYTES/2))).toBe(true)
    expect(retainedStepStoreAcceptsEncodedBytes('é'.repeat(MAX_RETAINED_STEP_STORE_BYTES/2)+'a')).toBe(false)
  })

  it('round-trips the authoritative retained model through project IndexedDB',async()=>{
    Object.defineProperty(globalThis,'indexedDB',{configurable:true,value:new IDBFactory()})
    const routed=importStepForWorkbench(fixture('self-authored-ap242-periodic-cylinder.step'))
    await saveProjectStepModel(routed.retainedModel!,routed.report)
    const reloaded=await loadProjectStepModel()
    expect(reloaded?.model.topologyIds).toEqual(routed.retainedModel?.topologyIds)
    expect(importDirectStepV6(exportDirectStepV6(reloaded!.model).text).model.bodies).toHaveLength(1)
  })
})
