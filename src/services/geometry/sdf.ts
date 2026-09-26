import {callGeometryRust} from './kernel'
import type {GeometryDeformation} from '../geometryEditing'
import {normalizePolygonMesh} from './polygon'
import type {PolygonBuild,PolygonMesh,PolygonProfile} from './polygon'
export type SdfProfile = PolygonProfile
export type SdfField =
 | {kind:'extrude';profile:SdfProfile;half_height:number}
 | {kind:'revolve';profile:SdfProfile}
 | {kind:'deform';input:SdfField;deformation:GeometryDeformation}
 | {kind:'mesh_distance';mesh:PolygonMesh;signed:boolean}
 | {kind:'sphere';center:number[];radius:number}
 | {kind:'box';center:number[];half_size:number[]}
 | {kind:'torus';center:number[];major_radius:number;minor_radius:number}
 | {kind:'capsule';a:number[];b:number[];radius:number}
 | {kind:'union'|'intersection'|'difference';a:SdfField;b:SdfField}
 | {kind:'smooth_union'|'smooth_difference';a:SdfField;b:SdfField;radius:number}
 | {kind:'offset';input:SdfField;distance:number}
 | {kind:'translate';input:SdfField;vector:number[]}
export interface SdfGrid {min:number[];max:number[];cells:number[]}
export function validateSdfBudget(field:SdfField):void {
 const pending:{field:SdfField;depth:number}[]=[{field,depth:0}];let count=0;let triangles=0;
 while(pending.length){const {field:f,depth}=pending.pop()!;if(++count>256||depth>32)throw new Error('SDF tree exceeds 256 nodes or depth 32');
 if(f.kind==='mesh_distance'&&(triangles+=f.mesh.indices.length/3)>4096)throw new Error('SDF mesh sources exceed 4096 triangles');
 if('a' in f&&f.kind!=='capsule')pending.push({field:f.a,depth:depth+1},{field:f.b,depth:depth+1});
 if('input' in f)pending.push({field:f.input,depth:depth+1});}
}
export const evaluateSdf=(field:SdfField,point:number[]):number=>{validateSdfBudget(field);return callGeometryRust('sdf_evaluate',{field,point})}
export const tessellateSdf=(field:SdfField,grid:SdfGrid):PolygonBuild=>{validateSdfBudget(field);return normalizePolygonMesh(callGeometryRust('sdf_tessellate',{field,grid}))}

/** Twist preserves the zero set through inverse mapping; distance is not certified. */
export const deformSdf=(field:SdfField,deformation:GeometryDeformation):SdfField=>{validateSdfBudget(field);return callGeometryRust('sdf_deform',{field,deformation})}
export type SdfTool=
 | {shape:'sphere';center:number[];radius:number}
 | {shape:'box';center:number[];half_size:number[]}
 | {shape:'capsule';a:number[];b:number[];radius:number}
/** One SDF sculpt stroke; `blend` is the smooth-blend radius (omit for hard CSG). */
export interface SdfStroke {tool:SdfTool;remove?:boolean;blend?:number}
export const sculptSdf=(field:SdfField,stroke:SdfStroke):SdfField=>{validateSdfBudget(field);if(stroke.blend!==undefined&&!(Number.isFinite(stroke.blend)&&stroke.blend>0))throw new Error('SDF sculpt blend radius must be positive.');return callGeometryRust('sdf_sculpt',{field,stroke})}
export const sculptSdfSphere=(field:SdfField,center:number[],radius:number,remove=false):SdfField=>{validateSdfBudget(field);return callGeometryRust('sdf_sculpt_sphere',{field,center,radius,remove})}

/** Browser WebGPU sweep support; see sdfGpu.ts. */
export interface SdfGpuJob {field:SdfField;grid:SdfGrid}
const gpuPending=new Map<string,{id:number,values:Float32Array}>()
const sdfKey=(field:SdfField,grid:SdfGrid)=>JSON.stringify([field,grid])
/** Prepares a GPU sweep for an eligible field; null when the kernel declines. */
export function prepareSdfGpu(field:SdfField,grid:SdfGrid):{id:number,payload:import('../sdfGpu').SdfGpuPayload}|null{
 const value=callGeometryRust<Record<string,unknown>|null>('sdf_prepare',{field,grid})
 if(value===null)return null
 return{id:value.id as number,payload:value as unknown as import('../sdfGpu').SdfGpuPayload}
}
/** Registers host-computed scores for the later synchronous tessellation. */
export function primeSdfGpu(job:SdfGpuJob,id:number,values:Float32Array):void{
 gpuPending.set(sdfKey(job.field,job.grid),{id,values})
}

/** Batch variant of prepareSdfGpu: one kernel call registers every eligible
 * job; ids mirror the input order with nulls for declined slots. */
export function prepareSdfGpuBatch(jobs:SdfGpuJob[]):{ids:(number|null)[],payload:import('../sdfGpu').SdfGpuBatchPayload}|null{
 if(!jobs.length)return null
 const value=callGeometryRust<{jobs:(Record<string,unknown>|null)[],wgsl:string,wgslVariants?:unknown[]}|null>('sdf_prepare_batch',{jobs})
 if(value===null)return null
 const ids=value.jobs.map(job=>job?(job.id as number):null)
 const payload:import('../sdfGpu').SdfGpuBatchPayload={
  jobs:value.jobs.map((job,index)=>job?{...job as Record<string,unknown>,id:ids[index]!} as unknown as import('../sdfGpu').SdfGpuJobPayload:null),
  wgsl:value.wgsl,
  ...(value.wgslVariants?{wgslVariants:value.wgslVariants as import('../webgpuFeatures').WgslVariant[]}:{}),
 }
 return{ids,payload}
}
/** Primes every sampled job for the later synchronous tessellation. */
export function primeSdfGpuBatch(jobs:SdfGpuJob[],ids:(number|null)[],values:(Float32Array|null)[]):void{
 ids.forEach((id,index)=>{
  if(id===null||!values[index])return
  gpuPending.set(sdfKey(jobs[index]!.field,jobs[index]!.grid),{id,values:values[index]!})
 })
}
export const tessellateSdfGpuAware=(field:SdfField,grid:SdfGrid):PolygonBuild=>{
 const key=sdfKey(field,grid)
 const primed=gpuPending.get(key)
 // encodeBinary reads typed numeric arrays elementwise exactly like the
 // equivalent plain array, so the primed grid crosses without an Array.from copy.
 if(primed){gpuPending.delete(key);return callGeometryRust('sdf_finish',{id:primed.id,values:primed.values})}
 return tessellateSdf(field,grid)
}
