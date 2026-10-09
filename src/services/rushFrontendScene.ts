import {compileRushGraphNurbs} from './rushGraphNurbsCompiler'
import {transformNurbsSurfacePatches} from './nurbsSurface'
import type {ProgressiveSweepPreview} from './nurbsConstructors'
/** Publish an own-kernel mesh directly to the viewer, without a SCAD/Manifold pass. */
import { buildOwnNurbsAsync, collectSdfJobs, type OwnNurbsBuildControl } from './rushGraphNurbsKernel'
import { prepareSdfGpuBatch, primeSdfGpuBatch } from './geometry/sdf'
import { runSdfSweepBatch } from './sdfGpu'
import type { GeometryEvaluationResult, GeometryQuality } from '../core/build'
import { geometryAssetId } from '../core/scene'
import { buildMeshBvh } from './meshBvh'
import { extractSemanticEdges } from './meshTopology'
export async function buildTextNurbsScene(document: unknown, quality: GeometryQuality = 'full', control:OwnNurbsBuildControl={}): Promise<GeometryEvaluationResult> {
  const start=performance.now()
  const scenePreview=control.onSweepPreview?progressiveScenePreviewMapper(document):null
  // GPU prefetch: eligible SDF grids are sampled by WebGPU before the
  // synchronous build — all grids in one session (one pipeline, one submit);
  // failures silently leave the CPU reference path.
  if (typeof navigator !== 'undefined' && navigator.gpu) {
    const jobs = collectSdfJobs(document)
    if (jobs.length) {
      try {
        const batch = prepareSdfGpuBatch(jobs)
        if (batch) primeSdfGpuBatch(jobs, batch.ids, await runSdfSweepBatch(batch.payload))
      } catch { /* CPU path at tessellation time */ }
    }
  }
  const result=await buildOwnNurbsAsync(document,{action:'build',display:{segments:quality==='preview'?8:24,subdivisionLevels:quality==='preview'?1:2}},{...control,onSweepPreview:async(id,preview)=>{
    const placed=scenePreview?.(id,preview)
    if(placed)await control.onSweepPreview?.(id,placed)
  }})
  if(!('mesh' in result) || !result.mesh)throw new Error('This geometry needs an explicit display conversion (SDF requires grid bounds; curves/profiles do not yet have a line renderer)')
  const mesh=result.mesh,evaluated=performance.now()
  const nativeGeometry='nativeGeometry' in result?result.nativeGeometry:undefined
  const entityId=`entity:native/${nativeGeometry?.nodeId??result.report.root}` as const
  const {mesh:output,surfaceArea}=ownNurbsDisplayMesh(mesh,entityId,nativeGeometry)
  return {meshes:mesh.indices.length?[output]:[],warnings:mesh.report.closed||!mesh.indices.length?[]:['Open surface: enclosed volume is undefined.'],volume:mesh.report.closed?Math.abs(mesh.report.signedVolumeMm3):0,surfaceArea,quality,reduced:false,timings:{parseMs:0,bindMs:0,initializeMs:0,evaluateMs:evaluated-start,analyzeMs:performance.now()-evaluated}}
}

/** Display-only adapter shared by final scenes and intermediate sweep previews. */
export function ownNurbsDisplayMesh(
 mesh:ReturnType<typeof import('./geometry/tessellation').tessellateNurbsSurface>,
 entityId:import('../core/mesh').MeshData['entityId'],
 nativeGeometry?:import('../core/nativeGeometry').NativeGeometryArtifact,
){
  const vertices=new Float32Array(mesh.indices.length*6),indices=new Uint32Array(mesh.indices.length)
  let surfaceArea=0
  for(let t=0;t<mesh.indices.length;t+=3) {
    const points=Array.from(mesh.indices.slice(t,t+3),i=>Array.from(mesh.positions.slice(i*3,i*3+3)))
    const a=points[1]!.map((x,i)=>x-points[0]![i]!),b=points[2]!.map((x,i)=>x-points[0]![i]!)
    const normal=[a[1]!*b[2]!-a[2]!*b[1]!,a[2]!*b[0]!-a[0]!*b[2]!,a[0]!*b[1]!-a[1]!*b[0]!],length=Math.hypot(...normal)
    surfaceArea+=length/2
    for(let j=0;j<3;j++){indices[t+j]=t+j;vertices.set([...points[j]!,...normal.map(x=>length?x/length:0)],(t+j)*6)}
  }
  const edges=extractSemanticEdges(vertices,indices)
  const output={vertices,indices,geometryAssetId:geometryAssetId(vertices,indices),entityId,...(nativeGeometry?{nativeGeometry}:{}),bvh:buildMeshBvh(vertices,indices),edgeIndices:edges.indices,topology:edges.diagnostics,color:[0.85,0.67,0.3,1] as [number,number,number,number],transform:new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]),faceIds:mesh.faceIds?new Uint32Array(mesh.faceIds):new Uint32Array(indices.length/3),faceIdsAuthoritative:!!mesh.faceIds||mesh.report.construction==='sampled_surface',provenance:[{triangleStart:0,triangleEnd:indices.length/3,source:null,backside:false}]}
  return {mesh:output,surfaceArea}
}

/** Preserves exact downstream affine placement before the display conversion.
 * Operations that change geometry require their own preview evaluator.
 */
export function progressiveScenePreviewMapper(document:unknown){
 const compiled=compileRushGraphNurbs(document)
 const nodes=new Map(compiled.resolved_document.nodes.map(node=>[node.id,node]))
 return (id:string,preview:ProgressiveSweepPreview):ProgressiveSweepPreview|null=>{
  let key=compiled.document.root
  const matrices:number[][][]=[]
  const seen=new Set<string>()
  while(key!==id){
   if(seen.has(key))return null
   seen.add(key)
   const node=nodes.get(key)
   if(!node)return null
   if(node.op==='transform'){matrices.push(node.matrix);key=node.input}
   else if(node.op==='nurbs_patches_tessellate'||node.op==='brep_tessellate')key=node.input
   else return null
  }
  let patches=preview.patches
  for(const matrix of matrices.reverse())patches=transformNurbsSurfacePatches(patches,matrix)
  return {...preview,patches}
 }
}
