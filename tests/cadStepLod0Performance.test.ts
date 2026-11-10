import {describe,expect,it} from 'vitest'
import {createStepLod0BenchmarkFixture} from '../benchmarks/step-lod0-fixture'
import {estimateDirectStepLod0Hausdorff} from '../src/kernel/cadNurbsStep'
import {importStepForWorkbench} from '../src/kernel/cadStepRouting'
import {MAX_LOD0_DURABLE_BYTES,MAX_LOD0_MEMORY_BYTES,stepLod0Cache} from '../src/services/stepLod0Cache'

const values=new Map<string,string>()
Object.defineProperty(globalThis,'localStorage',{configurable:true,value:{
  getItem:(key:string)=>values.get(key)??null,setItem:(key:string,value:string)=>{values.set(key,value)},removeItem:(key:string)=>{values.delete(key)},
  key:(index:number)=>[...values.keys()][index]??null,get length(){return values.size},
}})

describe('AP242 mixed exact + baked LOD-0 measured integration',()=>{
  it('reuses the direct mesh, respects budgets and labels BVH Hausdorff samples honestly',()=>{
    const fixture=createStepLod0BenchmarkFixture()
    stepLod0Cache.clear()
    const first=importStepForWorkbench(fixture.mixedText)
    const second=importStepForWorkbench(fixture.mixedText)
    expect(first.report.route).toBe('retained-ap242-brep')
    expect(first.retainedModel).toBeDefined()
    expect(first.lod0).toMatchObject({cacheReady:true,requiresRetessellation:false,exactGeometryAuthoritative:true})
    expect(second.lod0?.lod0Shells[0]).toBe(first.lod0?.lod0Shells[0])
    expect(Array.from(first.bodies[0].mesh.positions)).toEqual(fixture.lod0Mesh.positions)
    expect(Array.from(first.bodies[0].mesh.indices)).toEqual(fixture.lod0Mesh.indices)
    expect(stepLod0Cache.stats().bytes).toBeLessThanOrEqual(MAX_LOD0_MEMORY_BYTES)
    stepLod0Cache.reload()
    expect(importStepForWorkbench(fixture.mixedText).lod0?.cacheKey).toBe(first.lod0?.cacheKey)
    const durableBytes=[...values].filter(([key])=>key.startsWith('open-scad-viewer.step-lod0-v1.'))
      .reduce((sum,[,value])=>sum+new TextEncoder().encode(value).byteLength,0)
    expect(durableBytes).toBeLessThanOrEqual(MAX_LOD0_DURABLE_BYTES)

    const zero=estimateDirectStepLod0Hausdorff(fixture.exactMesh,fixture.lod0Mesh)
    expect(zero).toMatchObject({method:'two-sided-bvh-sampled-hausdorff-estimate',samplePattern:'vertices-edge-midpoints-centroids',certified:false})
    expect(zero.max).toBeLessThanOrEqual(1e-12)
    const shifted=createStepLod0BenchmarkFixture(0.125)
    const estimate=estimateDirectStepLod0Hausdorff(shifted.exactMesh,shifted.lod0Mesh)
    expect(estimate.max).toBeCloseTo(0.125,12)
    expect(estimate.sampleCount).toBe((fixture.exactMesh.positions.length/3+fixture.exactMesh.indices.length/3*4)*2)
  })
})
