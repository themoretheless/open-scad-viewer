import {expect,it} from 'vitest'
import {bezierNurbsCurve} from '../src/services/nurbsConstructors'
import {createRationalBrepSectionLoft} from '../src/services/geometry/brep'
import {inspectMiterProfileSmoothness} from '../src/services/miterProfileSmoothness'
import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

it('retains nonuniform station G2 when sharp rational profile joins refuse',()=>{
 const corners=[[-1,-1],[1,-1],[1,1],[-1,1],[-1,-1]]
 let z=0
 const sections=Array.from({length:49},(_,station)=>{
  if(station>0)z+=station%2===0?2:1
  return [corners.slice(0,-1).map((a,i)=>{
   const b=corners[i+1]!
   return {...bezierNurbsCurve([[a[0]!,a[1]!,z],[(a[0]!+b[0]!)*.5,(a[1]!+b[1]!)*.5,z],[b[0]!,b[1]!,z]]),weights:[1,.5,1]}
  })]
 })
 const model=createRationalBrepSectionLoft(sections),caps=[model.faces.length-2,model.faces.length-1]
 const before=JSON.stringify(model),proof=inspectMiterProfileSmoothness(model,caps)
 expect(model.faces).toHaveLength(194)
 expect(proof).toMatchObject({extractionComplete:true,profileG1Certified:false,
  stationContinuity:'G2',capContinuity:'C0',fullBoundarySmoothnessCertified:false,
  station:{extractionComplete:true,stationG1Certified:true,stationG2Certified:true}})
 expect(proof.profile.exactG1G2Certified).toBe(false)
 expect(proof.totalExactWork).toBeLessThanOrEqual(2000000)
 expect(proof.profile.seams).toHaveLength(proof.edgeIds.length)
 expect(proof.profile.unresolvedSeams).toHaveLength(proof.edgeIds.length)
 expect(proof.profile.seams.slice(1).every(s=>!s.certified&&!s.exactIdentity&&!s.regularityCertified&&s.work===0)).toBe(true)
 const short=inspectMiterProfileSmoothness(model,caps,proof.totalExactWork-1)
 expect(short.station.stationG2Certified).toBe(false)
 expect(short.totalExactWork).toBeLessThanOrEqual(proof.totalExactWork-1)
 expect(inspectMiterProfileSmoothness(model,caps,0)).toMatchObject({profileG1Certified:false,station:{stationG1Certified:false,stationG2Certified:false}})
 expect(JSON.stringify(model)).toBe(before)
})

it('keeps exact profile G1 curvature jumps separate from nonuniform station G2',()=>{
 const points=[[1,0],[0,1],[-1,0],[0,-1],[1,0]]
 const sections=[0,1,3].map(z=>[points.slice(0,-1).map((a,i)=>{
  const b=points[i+1]!
  return {...bezierNurbsCurve([[a[0]!,a[1]!,z],[a[0]!+b[0]!,a[1]!+b[1]!,z],[b[0]!,b[1]!,z]]),weights:[1,i===0?1:.5,1]}
 })])
 const model=createRationalBrepSectionLoft(sections),proof=inspectMiterProfileSmoothness(model,[model.faces.length-2,model.faces.length-1])
 expect(proof).toMatchObject({profileG1Certified:true,g1Method:'exact-projective-audit',
  stationContinuity:'G2',capContinuity:'C0',fullBoundarySmoothnessCertified:false,
  station:{stationG1Certified:true,stationG2Certified:true}})
 expect(proof.profile.exactG1G2Certified).toBe(false)
 expect(proof.g1Audit?.certifiedOrder).toBe(1)
})

it('carries sharp-profile/nonuniform-station G2 through ordinary Rush, viewport and fresh Solid',async()=>{
 const source=readFileSync('examples/rush/sharp-profile-nonuniform-station-g2-miter.r','utf8')
 const built=await buildOwnNurbsAsync(compileRushFrontend(source).document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing native retained rational miter body')
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({solidGeometryCertified:true,continuousBound:true,
  profileG1Certified:false,profileG2Certified:false,stationG1Certified:true,stationG2Certified:true,
  stationContinuity:'G2',capContinuity:'C0'})
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
 expect(model.faces).toHaveLength(194)
 const before=JSON.stringify(model),caps=[model.faces.length-2,model.faces.length-1]
 expect(inspectMiterProfileSmoothness(model,caps).station.stationG2Certified).toBe(true)
 const kink=JSON.parse(before)
 kink.faces[76].surface.controlPoints[1][1][0]=Number.EPSILON
 expect(inspectMiterProfileSmoothness(kink,caps).station.stationG2Certified).toBe(false)
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)).toMatchObject({solidGeometryCertified:true,
  boundaryEmbeddingCertified:true,allFacesInjective:true,allPairsClassified:true,nextPair:null})
 expect(JSON.stringify(model)).toBe(before)
})
