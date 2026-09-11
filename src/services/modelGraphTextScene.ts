/** Publish an own-kernel mesh directly to the viewer, without a SCAD/Manifold pass. */
import { buildOwnNurbs, collectSdfJobs } from './modelGraphNurbsKernel'
import { prepareSdfGpu, primeSdfGpu } from './geometry/sdf'
import { runSdfSweep } from './sdfGpu'
import type { GeometryEvaluationResult, GeometryQuality } from '../core/build'
import { geometryAssetId } from '../core/scene'
import { buildMeshBvh } from './meshBvh'
import { extractSemanticEdges } from './meshTopology'
export async function buildTextNurbsScene(document: unknown, quality: GeometryQuality = 'full'): Promise<GeometryEvaluationResult> {
  const start=performance.now()
  // GPU prefetch: eligible SDF grids are sampled by WebGPU before the
  // synchronous build; failures silently leave the CPU reference path.
  if (typeof navigator !== 'undefined' && navigator.gpu) {
    for (const job of collectSdfJobs(document)) {
      try {
        const prepared = prepareSdfGpu(job.field, job.grid)
        if (prepared) primeSdfGpu(job, prepared.id, await runSdfSweep(prepared.payload))
      } catch { /* CPU path at tessellation time */ }
    }
  }
  const result=buildOwnNurbs(document,{action:'build',display:{segments:quality==='preview'?8:24,subdivisionLevels:quality==='preview'?1:2}})
  if(!('mesh' in result) || !result.mesh)throw new Error('This geometry needs an explicit display conversion (SDF requires grid bounds; curves/profiles do not yet have a line renderer)')
  const mesh=result.mesh,evaluated=performance.now()
  const nativeGeometry='nativeGeometry' in result?result.nativeGeometry:undefined
  const entityId=`entity:native/${nativeGeometry?.nodeId??result.report.root}` as const
  const vertices=new Float32Array(mesh.indices.length*6),indices=new Uint32Array(mesh.indices.length)
  let surfaceArea=0
  for(let t=0;t<mesh.indices.length;t+=3) {
    const points=mesh.indices.slice(t,t+3).map(i=>mesh.positions.slice(i*3,i*3+3))
    const a=points[1]!.map((x,i)=>x-points[0]![i]!),b=points[2]!.map((x,i)=>x-points[0]![i]!)
    const normal=[a[1]!*b[2]!-a[2]!*b[1]!,a[2]!*b[0]!-a[0]!*b[2]!,a[0]!*b[1]!-a[1]!*b[0]!],length=Math.hypot(...normal)
    surfaceArea+=length/2
    for(let j=0;j<3;j++){indices[t+j]=t+j;vertices.set([...points[j]!,...normal.map(x=>length?x/length:0)],(t+j)*6)}
  }
  const edges=extractSemanticEdges(vertices,indices)
  const output={vertices,indices,geometryAssetId:geometryAssetId(vertices,indices),entityId,...(nativeGeometry?{nativeGeometry}:{}),bvh:buildMeshBvh(vertices,indices),edgeIndices:edges.indices,topology:edges.diagnostics,color:[0.85,0.67,0.3,1] as [number,number,number,number],transform:new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]),faceIds:mesh.faceIds?new Uint32Array(mesh.faceIds):new Uint32Array(indices.length/3),faceIdsAuthoritative:!!mesh.faceIds||mesh.report.construction==='sampled_surface',provenance:[{triangleStart:0,triangleEnd:indices.length/3,source:null,backside:false}]}
  return {meshes:indices.length?[output]:[],warnings:mesh.report.closed?[]:['Open surface: enclosed volume is undefined.'],volume:mesh.report.closed?Math.abs(mesh.report.signedVolumeMm3):0,surfaceArea,quality,reduced:false,timings:{parseMs:0,bindMs:0,initializeMs:0,evaluateMs:evaluated-start,analyzeMs:performance.now()-evaluated}}
}
