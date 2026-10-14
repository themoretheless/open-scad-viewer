import { PendingProgressiveBody, PendingProgressiveMiter, PendingProgressiveSweep } from '../rushGraphNurbsKernel'
import type { OwnNurbsBuildControl, OwnNurbsRequest } from '../rushGraphNurbsKernel'
import { createProgressiveBrepProfileBody, streamProgressiveBrepProfileBody, streamProgressiveMiterBrepProfileBody } from '../geometry/brep'
import type { ProgressiveMiterBrepBody } from '../geometry/brep'
import { progressiveSweepNurbsProfiles, streamProgressiveNurbsProfiles } from '../nurbsConstructors'
import type { ProgressiveSweepPreview } from '../nurbsConstructors'
import { buildOwnNurbsResolved } from './buildOwnNurbsResolved'

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
