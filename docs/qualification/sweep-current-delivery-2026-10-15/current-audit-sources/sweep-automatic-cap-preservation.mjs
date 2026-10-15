import assert from 'node:assert/strict'
import {checkAutomaticSmoothCapContract} from './sweep-automatic-cap-contracts.mjs'

/** Compare unchanged original walls and boundary edges, while explicitly
 * accounting for replacement of two legacy planar caps with eight native
 * radial patches. This receipt is separate from unchanged-model equality. */
export function checkAutomaticCapSourcePreservation(item,baseline,original){
 assert.equal(item.file,baseline.file);assert.equal(original.file,item.file)
 checkAutomaticSmoothCapContract(item)
 const changed=new Set(['faces','edges','capFaces','capHoleFaces'])
 for(const [field,value] of Object.entries(baseline.requestAndTopology))
  if(!changed.has(field))assert.deepEqual(item[field]??null,value,`${item.file}: original ${field} changed`)
 assert.equal(item.faces,original.faces+6)
 assert.equal(item.edges,original.edges+8)
 assert.deepEqual(item.capFaces,[]);assert.equal(item.capHoleFaces,0)
 const wallCount=original.faces-2,caps=new Set(item.physicalCapFaces)
 const walls=Array.from({length:item.faces},(_,id)=>id).filter(id=>!caps.has(id))
 assert.deepEqual(walls,Array.from({length:wallCount},(_,id)=>id),'Original wall ownership changed')
 for(const field of ['wallSurfaces','wallCoedges','wallFaceReversed']){
  assert.equal(original[field].length,wallCount)
  assert.deepEqual(item[field].slice(0,wallCount),original[field],`${item.file}: original ${field} changed`)
 }
 assert.deepEqual(item.faceLoops.slice(0,wallCount),original.faceLoops.slice(0,wallCount),'Original wall loop ownership changed')
 assert.deepEqual(item.edgeCurves.slice(0,original.edges).map(e=>e.curve),original.edgeCurves,'Original boundary edge changed')
 return {file:item.file,sourceWallsAndBoundaryEdgesPreserved:true,requestAndProofBudgetsPreserved:true,
  topologyChange:{legacyPlanarCaps:2,nativeRadialCaps:8,additionalFaces:6,additionalInternalEdges:8},
  sourceWallFaces:wallCount,sourceBoundaryEdges:original.edges,actualNativeSolid:true,
  scope:'Explicit native cap upgrade; unchanged original walls/edges, not unchanged cap interiors'}
}
