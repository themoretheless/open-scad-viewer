import {callGeometryRust} from './geometry/kernel'
import type {NativeGeometryArtifact} from '../core/nativeGeometry'
import type {NurbsBrep} from './geometry/brep'
import {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS,type SweepVolumeAudit} from './nurbsSweepEmbedding'
/** Recompute material evidence for the exact B-rep being transferred.
 * Snapshot certificate flags are deliberately not admission inputs. */
export function inspectProgressiveSweepSolidAdmission(artifact:NativeGeometryArtifact,model:NurbsBrep):SweepVolumeAudit|null {
 const document=JSON.parse(artifact.documentJson)
 const nodes=new Map<string,{id:string;op:string;input?:string;closed?:boolean}>(
  Array.isArray(document?.nodes)?document.nodes.map((node:{id:string;op:string})=>[node.id,node]):[])
 let node=nodes.get(artifact.nodeId)
 const visited=new Set<string>()
 // Affine B-rep transforms preserve face ownership, including endpoint caps.
 // Follow provenance only to choose the audit scope; recompute the certificate
 // on the transformed model rather than inheriting the source's positive flag.
 while(node?.op==='transform'||node?.op==='brep_smooth_miter_stations') {
  if(visited.has(node.id))throw new Error('Cyclic sweep transform provenance.')
  visited.add(node.id)
  if(typeof node.input!=='string'||!nodes.has(node.input))throw new Error('Missing sweep transform source.')
  node=nodes.get(node.input)
 }
 if(node?.op!=='brep_progressive_miter_sweep'&&node?.op!=='brep_miter_sweep'&&node?.op!=='brep_progressive_sweep')return null
 // Constructor closure chooses the native audit scope only. Every selected
 // cap and every other retained face is recomputed on the actual model; no
 // positive snapshot certificate is admitted. Missing scope uses all faces.
 const closed=node.op==='brep_progressive_sweep'
  ?JSON.parse(artifact.geometryJson)?.sweepBodyBoundaryEvidence?.closedPath
  :node.closed
 const caps=closed===false||node.op!=='brep_progressive_sweep'&&closed!==true
  ?[model.faces.length-2,model.faces.length-1]:[]
 const replay=JSON.parse(artifact.geometryJson)?.sweepMiterReplay
 const report: SweepVolumeAudit = replay
  ? (()=>{
    const result=callGeometryRust<{volume:SweepVolumeAudit|null;solidGeometryCertified:boolean;resultModelBound:boolean}>('brep_miter_owned_place',{
     ...replay,expectedResultModel:model,requireSolid:true,wallCells:100000,volumeBudgets:DEFAULT_SWEEP_VOLUME_BUDGETS})
    if(result.resultModelBound!==true)throw new Error('Progressive sweep replay requires native final-model binding.')
    if(!result.solidGeometryCertified)throw new Error('Progressive sweep replay Solid geometry could not be proved.')
    if(!result.volume)throw new Error('Progressive sweep replay material evidence missing.')
    return result.volume
   })()
  : inspectSweepVolume(model,caps,DEFAULT_SWEEP_VOLUME_BUDGETS)
 if(!report.solidGeometryCertified) {
  const stage=!report.boundaryEmbeddingCertified?'boundary embedding':report.nesting?.rolesConsistent!==true
   ?'shell nesting':'material orientation for shells '+report.orientations.filter(s=>s.outward!==s.expectedOutward).map(s=>s.shell).join(', ')
  throw new Error((node.op==='brep_miter_sweep'?'Miter sweep':'Progressive sweep')+' Solid geometry could not be proved: '+stage+'.')
 }
 return report
}
