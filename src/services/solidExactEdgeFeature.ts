import type {DirectBody} from './directModeling'
import {constantFilletFamily,exactConvexChamfer,exactAnnularFillet,exactLayeredPrismFillet,exactSimplePrismFillet,exactVariableRadiusFillet,exactValence3CornerBlend,tessellateNurbsBrep} from './geometry/brep'
import {assertBrepCapabilityAllowsTopology} from './geometry/brepCapability'
import {selectedBrepStraightEdge,solidTopology} from './directSolidTools'

/** Display edge indices are resolved uniquely to authored edges by the native kernel. */
export function solidExactEdgeFeature(body:DirectBody,displayEdges:number[],size:number,kind:'fillet'|'chamfer',source:'display'|'brep'='display',fillet:{mode:'constant'|'variable'|'corner';endRadius:number}={mode:'constant',endRadius:size}) {
 if(!body.brep)throw Error('Select a B-rep body for an exact edge feature.')
 const topology=source==='display'?solidTopology(body.mesh):null
 const edges=displayEdges.map(index=>{
  if(source==='brep'){
   if(!Number.isInteger(index)||!body.brep!.edges[index])throw Error('Select an existing B-rep edge.')
   return index
  }
  const edge=topology!.edges[index]
  if(!edge)throw Error('Select an existing straight edge before applying the feature.')
  return selectedBrepStraightEdge(body,[edge.a,edge.b])
 })
 const family=kind==='fillet'&&fillet.mode==='constant'?constantFilletFamily(body.brep,edges):'simple'
 const annular=family==='annular',layered=family==='layered'
 const capability=kind==='chamfer'?'exact-convex-straight-edge-chamfer/1':fillet.mode==='variable'?'exact-variable-radius-fillet/1':fillet.mode==='corner'?'exact-valence3-corner-blend/1':annular?'exact-annular-circular-edge-fillet/1':layered?'exact-layered-prism-edge-fillet/1':'exact-simple-prism-convex-edge-fillet/1'
 assertBrepCapabilityAllowsTopology(capability)
 const result=kind==='chamfer'?exactConvexChamfer(body.brep,edges,size):fillet.mode==='variable'?exactVariableRadiusFillet(body.brep,edges,[[size,fillet.endRadius]]):fillet.mode==='corner'?exactValence3CornerBlend(body.brep,edges,size):annular?exactAnnularFillet(body.brep,edges,size):layered?exactLayeredPrismFillet(body.brep,edges,size):exactSimplePrismFillet(body.brep,edges,size)
 if(result.certificate.capability!==capability||!result.certificate.complete||!result.audit.ok||!result.namingComplete)throw Error('The exact edge feature did not pass its geometry and identity audit.')
 return {body:{...body,brep:result.model,mesh:tessellateNurbsBrep(result.model,12)},evidence:result}
}
