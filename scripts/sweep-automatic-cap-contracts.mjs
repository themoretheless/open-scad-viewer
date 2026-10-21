import assert from 'node:assert/strict'
import {checkCompleteBoundaryContract} from './sweep-complete-boundary-contracts.mjs'

export const smoothCapGapCompanions=new Map([
 ['rmf-oblique-linear-scale-hollow.step','rmf-oblique-linear-scale-hollow-corrected.step'],
 ['frenet-quadratic-hollow.step','frenet-quadratic-hollow-corrected.step'],
 ['rmf-planar-quadratic-hollow.step','rmf-quadratic-hollow-corrected.step'],
])

// Receipt composition only. The global reconciler independently checks actual
// native face/pair coverage and externally imported STEP bytes for these cases.
export function checkAutomaticSmoothCapContract(item){
 assert.ok(smoothCapGapCompanions.has(item.file),'Unknown automatic-cap source contract')
 const body=item.construction?.bodyBoundaryEvidence
 assert.equal(body?.capRepresentation,'radial-annular','Original hollow must use native radial caps')
 assert.equal(body.budget,.01,'Original hollow tolerance changed')
 assert.equal(body.capCorrectionErrorUpper,null,'Original hollow must not receive plane correction')
 assert.equal(body.profileCorrectionErrorUpper,null,'Original hollow must not receive profile correction')
 assert.equal(item.requireNativeSolid,true,'Automatic-cap source must require actual Solid proof')
 assert.deepEqual(item.capFaces,[],'Radial caps must participate in every surface audit')
 assert.equal(item.physicalCapFaces?.length,8,'Missing physical radial patches')
 assert.deepEqual(body.capFaceGroups.map(group=>group.length),[4,4])
 assert.deepEqual(item.physicalCapFaces,body.capFaces)
 assert.equal(checkCompleteBoundaryContract(item)?.status,'certified','Original whole boundary not proved')
 const uses=item.nativeExactUses
 assert.ok(Array.isArray(uses)&&uses.length>0,'Missing actual coedge audits')
 assert.equal(item.faceLoops.length,item.faces,'Incomplete actual face-loop ownership')
 assert.ok(item.faceLoops.every(face=>face.outer.length===4&&face.holes.length===0),'Original radial model requires full rectangular patches')
 assert.equal(uses.length,4*item.faces,'Incomplete actual coedge audit')
 const seen=new Set()
 for(const use of uses){
  assert.ok(Number.isInteger(use.face)&&use.face>=0&&use.face<item.faces)
  assert.ok(Number.isInteger(use.coedge)&&use.coedge>=0&&use.coedge<4)
  const key=use.face+':'+use.coedge
  assert.ok(!seen.has(key),'Duplicated actual coedge audit');seen.add(key)
  assert.equal(use.edge,item.faceLoops[use.face].outer[use.coedge],'Coedge audit rebound to another edge')
 }
 assert.ok(uses.every(use=>use.status==='equal'&&use.exactIdentityCertified===true),'Automatic geometry contains an unproved coedge')
 assert.equal(item.nativeVolume?.solidGeometryCertified,true,'Actual automatic-cap global proof refused')
 return {file:item.file,representation:body.capRepresentation,physicalCapFaces:item.physicalCapFaces,
  boundaryErrorUpper:body.boundaryErrorUpper,budget:body.budget}
}
