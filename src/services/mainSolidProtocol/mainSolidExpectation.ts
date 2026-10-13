import { isNumericSequence } from '../mainSolidProtocol'
import type { MainSolidExpectation, MainSolidJob } from '../mainSolidProtocol'
import { boundaryExpectation } from '../solidBoundaryAgreement'
import { contactQualificationExpectation } from '../nurbsOffsetContactQualification'
import { envelopeFitExpectation } from '../nurbsOffsetEnvelopeFit'
import { faceContactExpectation } from '../solidFaceContacts'
import { materialExpectation } from '../solidMaterialVolume'
import { materialWallExpectation } from '../solidMaterialWall'
import { parseBondedSolidInput } from '../bondedSolidProtocol'
import { selfIntersectionExpectation } from '../solidSelfIntersection'
import { solidDistanceExpectation } from '../solidDistance'
import { sourceBodyExpectation } from '../sourceBody'
import { wholeWallExpectation } from '../solidWholeWall'

export function arrayOf(value:unknown, length:number, check:(v:unknown)=>boolean):boolean {
  if(!Array.isArray(value)||value.length!==length)return false
  for(const item of value)if(!check(item))return false
  return true
}

export function numericSequenceOf(value:unknown, length:number, check:(v:unknown)=>boolean):boolean {
  if(!isNumericSequence(value)||value.length!==length)return false
  for(let i=0;i<length;i++)if(!check(value[i]))return false
  return true
}

export function distanceWitnessConsistent(points:number[][],bounds:number[][][],lo:number,hi:number):boolean {
 const scale=Math.max(1,...points.flat().map(Math.abs),...bounds.flat(2).map(Math.abs),hi)
 const roundoff=Number.EPSILON*scale*32
 for(let side=0;side<2;side++)for(let k=0;k<3;k++)if(points[side][k]<bounds[side][k][0]-roundoff||points[side][k]>bounds[side][k][1]+roundoff)return false
 const gap=Math.hypot(...points[0].map((x,k)=>x-points[1][k]))
 const enclosureUpper=Math.hypot(...bounds[0].map((a,k)=>Math.max(Math.abs(a[0]-bounds[1][k][1]),Math.abs(a[1]-bounds[1][k][0]))))
 return Number.isFinite(gap)&&Number.isFinite(enclosureUpper)&&gap>=lo-roundoff&&gap<=hi+roundoff&&enclosureUpper<=hi+roundoff
}

export function mainSolidExpectation(job:MainSolidJob):MainSolidExpectation {
  if(job.kind==='sourceBodyRestore')return {kind:job.kind,...sourceBodyExpectation(job.options)}
  if(job.kind==='partialAnnularPreview')return {kind:job.kind,id:job.body.id,bodyIdentity:job.body.brep?.topologyIds?.bodies[0]}
  if(job.kind==='profileIntersections')return {kind:job.kind,loops:structuredClone(job.loops),options:{...job.options}}
  if(job.kind==='profilePrepare')return {kind:job.kind,chainSegments:job.ids.map(id=>{if(job.document.curves?.some(c=>c.id===id))return 1;const s=job.document.sketches.find(s=>s.id===id);return s?.analytic?.kind==='arc'?Math.ceil(Math.abs(s.analytic.sweep)/90):Math.max(0,(s?.points.length??0)-1)})}
  if(job.kind==='trimmedCurveOffset')return {kind:job.kind,createdId:job.options.createdId,fillRule:job.options.fillRule,toleranceMm:job.options.toleranceMm,intersectionToleranceMm:job.options.intersectionToleranceMm}
  if(job.kind==='wholeWall')return {kind:job.kind,...wholeWallExpectation(job.options)}
  if(job.kind==='materialWall')return {kind:job.kind,...materialWallExpectation(job.options)}
  if(job.kind==='materialSegment'||job.kind==='materialChord')return {kind:job.kind,...materialExpectation(job.options,job.kind==='materialChord'?'chord':'segment')}
  if(job.kind==='solidDistance')return {kind:job.kind,...solidDistanceExpectation(job.options)}
  if(job.kind==='selfIntersection')return {kind:job.kind,...selfIntersectionExpectation(job.model,job.toleranceUv,job.limits,job.maxSpans,job.boundaryAudit)}
  if(job.kind==='faceContacts')return {kind:job.kind,...faceContactExpectation(job.model,job.toleranceUv,job.limits)}
  if(job.kind==='boundaryAgreement')return {kind:job.kind,...boundaryExpectation(job.model,job.maxCells)}
  if(job.kind==='shellDistance'){const o=job.options;return {kind:job.kind,domains:[o.a,o.b].map(m=>m.faces.map(({surface:s})=>[[s.knotsU[s.degreeU],s.knotsU[s.controlPoints.length]],[s.knotsV[s.degreeV],s.knotsV[s.controlPoints[0]?.length??0]]])),toleranceMm:o.toleranceMm,toleranceUv:o.toleranceUv,maxCells:o.maxCells,maxDomainCells:o.maxDomainCells}}
  if(job.kind==='faceDistance'){const o=job.options;return {kind:job.kind,domains:[o.a.faces[o.faceA]?.surface,o.b.faces[o.faceB]?.surface].map(s=>s?[[s.knotsU[s.degreeU],s.knotsU[s.controlPoints.length]],[s.knotsV[s.degreeV],s.knotsV[s.controlPoints[0]?.length??0]]]:[]),toleranceMm:o.toleranceMm,toleranceUv:o.toleranceUv,maxCells:o.maxCells,maxDomainCells:o.maxDomainCells}}
  if(job.kind==='offsetContactQualification')return {kind:job.kind,...contactQualificationExpectation(job.options)}
  if(job.kind==='offsetEnvelopeFit')return {kind:job.kind,...envelopeFitExpectation(job.options)}
  if(job.kind==='offsetEnvelope') {
    const tangent=mainSolidExpectation({kind:'offsetContactTangent',options:job.options})
    if(tangent.kind!=='offsetContactTangent')throw Error('Invalid envelope expectation')
    return {kind:job.kind,tangent,radiusMm:Math.abs(job.options.distances[0]),maxCells:job.options.maxCells}
  }
  if(job.kind==='offsetContactTangent') {
    const band=mainSolidExpectation({kind:'offsetContactBand',options:job.options})
    if(band.kind!=='offsetContactBand')throw Error('Invalid tangent expectation')
    return {...band,kind:job.kind}
  }
  if(job.kind==='offsetSourceBoundary') {
    const o=job.options,trim=mainSolidExpectation({kind:'trimmedOffsetContactBand',options:o})
    if(trim.kind!=='trimmedOffsetContactBand')throw Error('Invalid boundary expectation')
    return {kind:job.kind,trim,addresses:[o.firstLoops,o.secondLoops].flatMap((loops,side)=>loops.flatMap((loop,li)=>loop.map((_,ci)=>[side,li,ci] as [number,number,number]))),toleranceMm:o.toleranceMm,maxExactWork:o.maxExactWork,maxAgreementCells:o.maxAgreementCells}
  }
  if(job.kind==='trimmedOffsetContactBand') {
    const o=job.options,band=mainSolidExpectation({kind:'offsetContactBand',options:o})
    if(band.kind!=='offsetContactBand')throw Error('Invalid contact expectation')
    return {...band,kind:job.kind,maxPairs:o.maxPairs,maxCells:o.maxCells,maxDomainCells:o.maxDomainCells,loopCounts:[o.firstLoops.length,o.secondLoops.length]}
  }
  if(job.kind==='offsetContactBand') {
    const o=job.options,first:[ [number,number],[number,number] ]=[o.firstOther,o.firstOther]
    first[o.fixedAxis]=o.fixedInterval
    return {kind:job.kind,fixedAxis:o.fixedAxis,domains:[first.map(r=>[r[0],r[1]]) as typeof first,o.secondDomain.map(r=>[r[0],r[1]]) as typeof first]}
  }
  if(job.kind==='surfaceDistance')return {kind:job.kind,domains:[job.a,job.b].map(s=>[[s.knotsU[s.degreeU],s.knotsU[s.controlPoints.length]],[s.knotsV[s.degreeV],s.knotsV[s.controlPoints[0]?.length??0]]]),toleranceMm:job.toleranceMm,maxCells:job.maxCells}
  if(job.kind==='curveDistance')return {kind:job.kind,dimension:job.a.controlPoints[0]?.length??0,domains:[job.a,job.b].map(c=>[c.knots[c.degree],c.knots[c.controlPoints.length]]),toleranceMm:job.toleranceMm,maxCells:job.maxCells}
  if(job.kind==='sketchSnaps'){const s=job.sketch,n=s.retainedProfile?s.retainedProfile.loops.reduce((sum,loop)=>sum+loop.length,0):s.points.length;return {kind:job.kind,maxPoints:s.retainedProfile?n*3:s.analytic?5:n*2+2,maxSegments:s.retainedProfile?s.retainedProfile.loops.flat().reduce((sum,c)=>sum+(c.controlPoints.length-c.degree)*(c.degree===1?1:16),0):s.analytic?0:n,maxCircles:!s.retainedProfile&&s.analytic?1:0}}
  if(job.kind==='bodySnaps'){const edges=job.body.brep?.edges.length??job.body.mesh.indices.length;return {kind:job.kind,maxPoints:edges*4+job.body.mesh.indices.length/3+1,maxSegments:edges*16}}
  if(job.kind==='bodyEdges')return {kind:job.kind,vertices:job.body.brep?.vertices.length??job.body.mesh.positions.length/3,edges:job.body.brep?.edges.length??job.body.mesh.indices.length,exact:!!job.body.brep}
  if(job.kind==='topology')return {kind:job.kind,vertices:job.mesh.positions.length/3,triangles:job.mesh.indices.length/3}
  if(job.kind==='curveChainInspection')return {kind:job.kind,maxPairs:job.maxPairs,segments:job.ids.reduce((n,id)=>n+(job.document.curves?.find(c=>c.id===id)?.curve.controlPoints.length??1)-1,0)}
  if(job.kind==='curveDisplay')return {kind:job.kind,dimension:job.curve.controlPoints[0]?.length??0}
  if(job.kind==='profileDisplay')return {kind:job.kind,counts:job.profile.loops.map(loop=>loop.reduce((n,c)=>n+(c.degree===1?1:24),0))}
  if(job.kind==='surfaceMesh')return {kind:job.kind,u:job.item.segmentsU,v:job.item.segmentsV}
  if(job.kind==='surfaceBoundary')return {kind:job.kind,samples:job.options.samples}
  if(job.kind==='displayMesh')return {kind:job.kind,triangles:job.mesh.indices.length/3}
  if(job.kind==='brepTool')return {kind:job.kind,id:job.options.id,operation:job.options.kind}
  if(job.kind==='meshContacts')return {kind:job.kind,triangles:job.mesh.indices.length/3,maxWork:job.maxWork??200_000,maxContacts:job.maxContacts??10_000}
  if(job.kind==='bondedSolid'){
    const m=parseBondedSolidInput(job.inputJson)
    return {kind:job.kind,nodes:m.nodesMm.length,tets:m.tets.length,bonds:m.bonds.length}
  }
  return job.kind==='truss' ? {kind:job.kind,nodes:job.model.nodesMm.length,members:job.model.members.length} : job.kind==='structuralSections'?{kind:job.kind,axis:job.axis,stations:[...job.stations]}:{kind:job.kind}
}
