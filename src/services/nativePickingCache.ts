/** Renderer-owned native snapshots, keyed by the shared GPU vertex buffer. */
import {callGeometryRust,GeometryKernelError} from './geometry/kernel'
import {createPickingSnapshotInKernel} from './geometry/meshAnalysis'
import type {BvhRay,MeshBvhHit,RaycastMeshBvhOptions} from './meshBvh'

export class NativePickingCache {
  private readonly handles=new Map<object,string>()
  constructor(private readonly capacity=32) {
    if(!Number.isInteger(capacity)||capacity<1||capacity>64)throw new RangeError('Invalid picking cache capacity')
  }
  release(key:object):void {
    const handle=this.handles.get(key)
    if(handle===undefined)return
    callGeometryRust('mesh_picking',{action:'dispose',handle})
    this.handles.delete(key)
  }
  clear():void {for(const key of this.handles.keys())this.release(key)}
  query(key:object,vertices:Float32Array,indices:Uint32Array,stride:number,leafSize:number,ray:BvhRay,options:Pick<RaycastMeshBvhOptions,'minT'|'excludedTriangles'|'localFromWorld'>={}):MeshBvhHit|null {
    if(![...ray.origin,...ray.direction].every(Number.isFinite))return null
    if(options.minT!==undefined&&!Number.isFinite(options.minT))throw new RangeError('Picking continuation must be finite')
    let handle=this.handles.get(key)
    if(handle===undefined){
      if(this.handles.size>=this.capacity)this.release(this.handles.keys().next().value!)
      for(;;){
        try{handle=createPickingSnapshotInKernel(vertices,indices,stride,leafSize);break}
        catch(error){
          // Only registry capacity refusal permits evicting another owned snapshot.
          if(!(error instanceof GeometryKernelError)||error.message!=='Native picking snapshot budget exceeded'||!this.handles.size)throw error
          this.release(this.handles.keys().next().value!)
        }
      }
    }
    this.handles.delete(key);this.handles.set(key,handle)
    // Typed-array/list exclusions pass straight to the binary encoder; only a
    // legacy Set needs materializing, so depth-cycling continuations can reuse
    // one growing buffer instead of rebuilding an array per query.
    const excluded=options.excludedTriangles
    return callGeometryRust<MeshBvhHit|null>('mesh_picking',{action:'query',handle,
      origin:ray.origin,direction:ray.direction,minT:options.minT??null,
      excludedTriangles:excluded===undefined?[]:typeof (excluded as ReadonlySet<number>).has==='function'?Array.from(excluded as ReadonlySet<number>):excluded,
      localFromWorld:options.localFromWorld?Array.from(options.localFromWorld):null})
  }
}
