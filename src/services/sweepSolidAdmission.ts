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
 while(node?.op==='transform') {
  if(visited.has(node.id))throw new Error('Cyclic sweep transform provenance.')
  visited.add(node.id)
  if(typeof node.input!=='string'||!nodes.has(node.input))throw new Error('Missing sweep transform source.')
  node=nodes.get(node.input)
 }
 if(node?.op!=='brep_progressive_miter_sweep'&&node?.op!=='brep_miter_sweep')return null
 const caps=node.closed===true?[]:[model.faces.length-2,model.faces.length-1]
 const report=inspectSweepVolume(model,caps,DEFAULT_SWEEP_VOLUME_BUDGETS)
 if(!report.solidGeometryCertified) {
  const stage=!report.boundaryEmbeddingCertified?'boundary embedding':report.nesting?.rolesConsistent!==true
   ?'shell nesting':'material orientation for shells '+report.orientations.filter(s=>s.outward!==s.expectedOutward).map(s=>s.shell).join(', ')
  throw new Error((node.op==='brep_progressive_miter_sweep'?'Progressive sweep':'Miter sweep')+' Solid geometry could not be proved: '+stage+'.')
 }
 return report
}
