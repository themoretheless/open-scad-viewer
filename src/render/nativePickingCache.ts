/** Renderer-owned native snapshots, keyed by the shared GPU vertex buffer. */
import {callGeometryRust,GeometryKernelError} from '../kernel/geometry/kernel'
import {createPickingSnapshotInKernel} from '../kernel/geometry/meshAnalysis'
import type {BvhRay,MeshBvhHit,RaycastMeshBvhOptions} from '../kernel/meshBvh'

export interface PickingNeighbor {
  readonly triangleIndex:number
  readonly distance:number
  readonly point:[number,number,number]
}

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
  /** Snapshot handle for `key`, uploading on first use and refreshing LRU order. */
  private acquire(key:object,vertices:Float32Array,indices:Uint32Array,stride:number,leafSize:number):string {
    let handle:string|undefined=this.handles.get(key)
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
    return handle
  }
  /** Triangles whose geometry intersects the closed box [min, max] in local coordinates. */
  overlap(key:object,vertices:Float32Array,indices:Uint32Array,stride:number,leafSize:number,min:readonly number[],max:readonly number[]):number[] {
    if(min.length!==3||max.length!==3||![...min,...max].every(Number.isFinite))throw new RangeError('Picking box must be finite')
    const handle=this.acquire(key,vertices,indices,stride,leafSize)
    return callGeometryRust<{triangles:number[]}>('mesh_picking',{action:'overlap',handle,min,max}).triangles
  }
  /** Triangles within `radius` of `center`, nearest first, with their closest points. */
  withinRadius(key:object,vertices:Float32Array,indices:Uint32Array,stride:number,leafSize:number,center:readonly number[],radius:number):PickingNeighbor[] {
    if(center.length!==3||!center.every(Number.isFinite)||!(radius>=0)||!Number.isFinite(radius))throw new RangeError('Picking radius query must be finite')
    const handle=this.acquire(key,vertices,indices,stride,leafSize)
    return callGeometryRust<{neighbors:PickingNeighbor[]}>('mesh_picking',{action:'withinRadius',handle,center,radius}).neighbors
  }
  /** Up to `k` (at most 4096) nearest triangles to `point`, nearest first. */
  nearest(key:object,vertices:Float32Array,indices:Uint32Array,stride:number,leafSize:number,point:readonly number[],k:number):PickingNeighbor[] {
    if(point.length!==3||!point.every(Number.isFinite)||!Number.isInteger(k)||k<1||k>4096)throw new RangeError('Nearest-triangle query needs a finite point and 1..4096 results')
    const handle=this.acquire(key,vertices,indices,stride,leafSize)
    return callGeometryRust<{neighbors:PickingNeighbor[]}>('mesh_picking',{action:'nearest',handle,point,k}).neighbors
  }
  query(key:object,vertices:Float32Array,indices:Uint32Array,stride:number,leafSize:number,ray:BvhRay,options:Pick<RaycastMeshBvhOptions,'minT'|'excludedTriangles'|'localFromWorld'>={}):MeshBvhHit|null {
    if(![...ray.origin,...ray.direction].every(Number.isFinite))return null
    if(options.minT!==undefined&&!Number.isFinite(options.minT))throw new RangeError('Picking continuation must be finite')
    const handle=this.acquire(key,vertices,indices,stride,leafSize)
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
