import { PendingProgressiveBody, PendingProgressiveMiter, PendingProgressiveSweep } from '../rushGraphNurbsKernel'
import type { OwnNurbsBuildControl, OwnNurbsRequest, SdfTessellationJob } from '../rushGraphNurbsKernel'
import { createProgressiveBrepProfileBody, streamProgressiveBrepProfileBody, streamProgressiveMiterBrepProfileBody } from '../geometry/brep'
import type { ProgressiveMiterBrepBody } from '../geometry/brep'
import { progressiveSweepNurbsProfiles, streamProgressiveNurbsProfiles } from '../nurbsConstructors'
import type { ProgressiveSweepPreview } from '../nurbsConstructors'
import type { SdfField } from '../geometry/sdf'
import { buildOwnNurbsResolved } from './buildOwnNurbsResolved'
import { compileRushGraphNurbs } from '../rushGraphNurbsCompiler'

/** Collects sdf_tessellate jobs resolvable without mesh evaluation, so the
 * caller can run their grid sampling on the GPU before the synchronous build. */
export function collectSdfJobs(document: unknown): SdfTessellationJob[] {
    type SdfField = import('../geometry/sdf').SdfField
    const compiled = compileRushGraphNurbs(document)
    const nodes = new Map(compiled.resolved_document.nodes.map(n => [n.id, n] as const))
    const memo = new Map<string, SdfField | null>()
    const pure = (key: string): SdfField | null => {
        const hit = memo.get(key)
        if (hit !== undefined) return hit
        const n = nodes.get(key) as any
        const field: SdfField | null = !n ? null
            : n.op === 'sdf_sphere' ? { kind: 'sphere', center: n.center, radius: n.radius }
            : n.op === 'sdf_box' ? { kind: 'box', center: n.center, half_size: n.half_size }
            : n.op === 'sdf_torus' ? { kind: 'torus', center: n.center, major_radius: n.major_radius, minor_radius: n.minor_radius }
            : n.op === 'sdf_union' || n.op === 'sdf_intersection' || n.op === 'sdf_difference'
                ? (() => { const a = pure(n.inputs[0]); const b = pure(n.inputs[1]); return a && b
                    ? { kind: n.op === 'sdf_union' ? 'union' : n.op === 'sdf_intersection' ? 'intersection' : 'difference', a, b } : null })()
            : n.op === 'sdf_smooth_union'
                ? (() => { const a = pure(n.inputs[0]); const b = pure(n.inputs[1]); return a && b ? { kind: 'smooth_union', a, b, radius: n.radius } : null })()
            : n.op === 'sdf_offset' ? (() => { const input = pure(n.input); return input ? { kind: 'offset', input, distance: n.distance } : null })()
            : n.op === 'sdf_translate' ? (() => { const input = pure(n.input); return input ? { kind: 'translate', input, vector: n.vector } : null })()
            : null
        memo.set(key, field)
        return field
    }
    const jobs: SdfTessellationJob[] = []
    for (const n of compiled.resolved_document.nodes) {
        if ((n as any).op !== 'sdf_tessellate') continue
        const node = n as any
        const field = pure(node.input)
        if (field) jobs.push({ field, grid: { min: node.min, max: node.max, cells: node.cells } })
    }
    return jobs
}

export function buildOwnNurbs(document:unknown,request:OwnNurbsRequest){
 return buildOwnNurbsResolved(document,request)
}

/** Keeps graph evaluation synchronous except for bounded progressive sweep levels. */
export async function buildOwnNurbsAsync(document:unknown,request:OwnNurbsRequest,control:OwnNurbsBuildControl={}){
 const completed=new Map<string,ReturnType<typeof progressiveSweepNurbsProfiles>>()
 const bodies=new Map<string,ReturnType<typeof createProgressiveBrepProfileBody>>()
 const miters=new Map<string,ProgressiveMiterBrepBody>()
 const checkAbort=()=>{if(control.shouldAbort?.())throw new DOMException('Build cancelled','AbortError')}
 async function consume<T>(nodeId:string,stream:AsyncGenerator<ProgressiveSweepPreview,T,void>):Promise<T>{
  try{
   for(;;){
    const level=await stream.next()
    checkAbort()
    control.onYield?.()
    if(level.done)return level.value
    await control.onSweepPreview?.(nodeId,level.value)
   }
  }finally{await stream.return(undefined as never)}
 }
 for(;;){
  checkAbort()
  try{
   return buildOwnNurbsResolved(document,request,(nodeId,args)=>{
    const ready=completed.get(nodeId)
    if(ready)return ready
    throw new PendingProgressiveSweep(nodeId,args)
   },(nodeId,args)=>{
    const ready=bodies.get(nodeId)
    if(ready)return ready
    throw new PendingProgressiveBody(nodeId,args)
   },(nodeId,args)=>{const ready=miters.get(nodeId);if(ready)return ready;throw new PendingProgressiveMiter(nodeId,args)})
  }catch(error){
   if(error instanceof PendingProgressiveSweep){
    completed.set(error.nodeId,await consume(error.nodeId,
     streamProgressiveNurbsProfiles(...error.args,{shouldAbort:control.shouldAbort})))
   }else if(error instanceof PendingProgressiveBody){
    bodies.set(error.nodeId,await consume(error.nodeId,
     streamProgressiveBrepProfileBody(...error.args,{shouldAbort:control.shouldAbort})))
   }else if(error instanceof PendingProgressiveMiter){
    miters.set(error.nodeId,await consume(error.nodeId,streamProgressiveMiterBrepProfileBody(...error.args,{shouldAbort:control.shouldAbort})))
   }else throw error
  }
 }
}
