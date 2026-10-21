import {expect,it} from 'vitest'
import {readAnnularG2Evidence} from '../src/services/annularG2Evidence'
// Receipt/presentation fixture only; this never supplies geometry admission.
function receipt(){return {geometricMaxCells:128,geometricCells:40,capInternalSeams:[0,1].map(cap=>{
 const edgeIds=[0,1,2,3].map(i=>cap*4+i)
 return {extractionComplete:true,edgeIds,g1Certified:false,g2Certified:false,geometricG2:{
  method:'owned-boundary-geometric-G2-intervals',complete:true,withinTolerance:true,exactIdentityClaim:false,solidGeometryCertified:false,
  positionTolerance:0,normalTolerance:1e-7,curvatureTolerance:1e-5,maxCells:cap?108:128,cells:20,edgeIds,
  seams:edgeIds.map((edge,i)=>({edge,faces:[16+cap*4+i,16+cap*4+(i+1)%4],inspection:{positionBounds:[0,0],regularityProven:true,cells:5,
   decision:'within-tolerance',orientation:'same',branches:[{normalLower:0,normalUpper:2e-12,curvatureLower:0,curvatureUpper:4e-9},{normalLower:1.99,normalUpper:2,curvatureLower:0,curvatureUpper:4e-9}]}}))
 }}
})}}
it('presents whole-seam tolerance G2 independently of exact strip G2',()=>{
 expect(readAnnularG2Evidence(receipt())).toEqual({capGeometricG2Certified:true,capGeometricSeamCount:8,capNormalChordUpper:2e-12,capCurvatureTensorUpper:4e-9})
 expect(readAnnularG2Evidence({})).toEqual({})
})
it('refuses incomplete scopes, inconsistent budgets and unsupported claims',()=>{
 const mutations=[(r:any)=>r.geometricCells++,(r:any)=>r.geometricMaxCells=0,(r:any)=>r.capInternalSeams[0].geometricG2.seams.pop(),
  (r:any)=>r.capInternalSeams[0].geometricG2.solidGeometryCertified=true,(r:any)=>r.capInternalSeams[0].geometricG2.exactIdentityClaim=true,
  (r:any)=>r.capInternalSeams[0].geometricG2.withinTolerance=false,(r:any)=>r.capInternalSeams[1].extractionComplete=false,
  (r:any)=>r.capInternalSeams[0].geometricG2.seams[0].inspection.orientation=null,
  (r:any)=>r.capInternalSeams[0].geometricG2.seams[0].inspection.cells=0,
  (r:any)=>r.capInternalSeams[0].geometricG2.seams[0].inspection.branches[0].normalUpper=1e-4,
  (r:any)=>r.capInternalSeams[0].geometricG2.seams[0].inspection.branches[0].curvatureUpper=1e-3,
  (r:any)=>r.capInternalSeams[0].geometricG2.seams[0].inspection.branches[0].normalLower=1,
  (r:any)=>r.capInternalSeams[0].geometricG2.seams[0].faces=[16,16]]
 for(const mutate of mutations){const r=receipt();mutate(r);expect(readAnnularG2Evidence(r).capGeometricG2Certified).toBe(false)}
})
