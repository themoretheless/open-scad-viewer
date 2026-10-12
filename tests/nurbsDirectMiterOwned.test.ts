import {expect,it} from 'vitest'
import {circleNurbsCurve,miterNurbsProfileSections} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {constructMiterOwnedBoundary,type DirectMiterOwnedSourceRequest} from '../src/services/nurbsMiterOwnedBoundary'
import {createMiterBrepProfileBody,createRationalBrepSectionLoft,transformNurbsBrep,type NurbsBrep} from '../src/services/geometry/brep'
import {projectSweepSections} from '../src/services/nurbsSectionProjection'
import {callGeometryRust} from '../src/services/geometry/kernel'
import {inspectMiterProfileSmoothness} from '../src/services/miterProfileSmoothness'

const points:[number,number,number][]=[[0,0,0],[0,0,10],[10,0,10],[10,10,15]]
const rings=(axis:[number,number,number]=[0,0,1])=>[[circleNurbsCurve([0,0,0],axis,.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],axis,.2))]]
const source=(loops=rings(),path=points):DirectMiterOwnedSourceRequest=>({direct:true,loops,points:path,normal:[1,0,0],closed:false,miterLimit:2,maxDeviation:.01})
const assertOwner=(request:DirectMiterOwnedSourceRequest)=>{
 const before=JSON.stringify(request),body=constructMiterOwnedBoundary(request)
 expect(JSON.stringify(request)).toBe(before)
 expect(body.retentionMode).toBe('direct')
 expect(body).toMatchObject({solidGeometryCertified:false,globalEmbeddingCertified:false})
 return body
}
it('binds full straight direct bound to unchanged legacy walls and filled caps',()=>{
 const request=source(rings(),points.slice(0,2)),body=assertOwner(request)
 expect(body.model).toEqual(createMiterBrepProfileBody(request.loops,request.points,request.normal,2).model)
 expect(body.sectionCorrection).toBeNull()
 expect(body.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,closed:false})
 expect(body.boundaryCertificate.filledCapErrorUpper).toHaveLength(2)
})
it('owns the actual closed planar legacy shell without inventing caps',()=>{
 const request:DirectMiterOwnedSourceRequest={...source(rings([1,0,0]),[[0,0,0],[10,0,0],[10,10,0],[0,10,0]]),normal:[0,0,1],closed:true}
 const body=assertOwner(request)
 expect(body.model).toEqual(createMiterBrepProfileBody(request.loops,request.points,request.normal,2,true).model)
 expect(body.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,closed:true,filledCapErrorUpper:null})
})
it('keeps raw spatial geometry and refuses its missing filled-cap proof',()=>{
 const request=source(),body=assertOwner(request)
 expect(body.model).toEqual(createMiterBrepProfileBody(request.loops,request.points,request.normal,2).model)
 expect(body.boundaryCertificate.continuousBound).toBe(false)
 expect(body.boundaryCertificate.errorUpper).toBeNull()
 expect(body.sectionCorrection).toBeNull()
})
it('owns explicit spatial correction without replacing the legacy corrected model',()=>{
 const capCorrection={quantum:2**-40,tolerance:1e-9,maxWork:1000000}
 const request={...source(),capCorrection},body=assertOwner(request)
 expect(body.model).toEqual(createMiterBrepProfileBody(request.loops,request.points,request.normal,2,false,capCorrection).model)
 expect(body.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(body.boundaryCertificate.filledCapErrorUpper).toHaveLength(2)
 expect(()=>constructMiterOwnedBoundary({...request,capCorrection:{...capCorrection,maxWork:0}})).toThrow(/correction unproved/)
})
it('replays the last-only external cap correction without modifying the starting cap',()=>{
 const capCorrection={quantum:2**-40,tolerance:1e-9,maxWork:1000000}
 const request:DirectMiterOwnedSourceRequest={...source(),capCorrection,capEndpoints:[false,true]}
 const direct=miterNurbsProfileSections(request.loops.flat(),request.points,request.normal,2)
 const correction=projectSweepSections(direct,[{section:direct.length-1,plane:{axis:1,coefficients:[0,-.5],offset:17.5},quantum:2**-40,tolerance:1e-9}],1000000)
 const expected=createRationalBrepSectionLoft(correction.sections!.map(row=>row.map(curve=>[curve])))
 const body=assertOwner(request)
 expect(body.model).toEqual(expected)
 expect(body.sections[0]!.flat()).toEqual(direct[0])
 expect(body.capEndpoints).toEqual([false,true])
 expect(body.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
})
const quarters=(r:number):NurbsCurve[]=>[
 [[r,0],[r,r],[0,r]],[[0,r],[-r,r],[-r,0]],
 [[-r,0],[-r,-r],[0,-r]],[[0,-r],[r,-r],[r,0]],
].map(p=>({degree:2,knots:[0,0,0,1,1,1],controlPoints:p.map(([x,y])=>[x!,y!,0]),weights:[1,Math.SQRT1_2,1],periodic:false}))
it.each(['circle','dyadic'] as const)('replays %s direct source for unchanged affine geometry and complete cap bound',kind=>{
 const loops=kind==='circle'?rings():[quarters(.5),quarters(.25).reverse().map(reverseNurbsCurve)]
 const request=source(loops,points.slice(0,2)),body=assertOwner(request)
 const matrix=[[1,0,0,0],[0,1,0,0],[1,1,1,0],[0,0,0,1]]
 const expected=transformNurbsBrep(body.model,matrix)
 const parameters={source:request,matrix,quantum:2**-40,maxWork:100000,budget:.01,
  expectedSourceModel:body.model,expectedSourceCertificate:body.boundaryCertificate,expectedResultModel:expected}
 const result=callGeometryRust<{placement:{model:NurbsBrep}|null;boundaryCertificate:{continuousBound:boolean;withinBudget:boolean;filledCapErrorUpper:number[]|null};solidGeometryCertified:boolean;resultModelBound:boolean}>('brep_miter_owned_place',parameters)
 expect(result.placement!.model).toEqual(expected)
 expect(result.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(result.boundaryCertificate.filledCapErrorUpper).toHaveLength(2)
 expect(result.solidGeometryCertified).toBe(false)
 expect(result.resultModelBound).toBe(true)
 expect(callGeometryRust<{placement:null}>('brep_miter_owned_place',{...parameters,maxWork:0}).placement).toBeNull()
})

it('keeps accepted spatial direct holonomy out of the source while charging retained closure',()=>{
 const request:DirectMiterOwnedSourceRequest={...source(rings([1,0,0]),[[0,0,0],[10,0,0],[13,9,4e-12],[-2,11,1e-12],[1,5,-2e-12]]),normal:[0,0,1],closed:true,miterLimit:4}
 const body=assertOwner(request)
 expect(body.model).toEqual(createMiterBrepProfileBody(request.loops,request.points,request.normal,4,true).model)
 expect(body.levels[0]!.holonomyCorrectionRadians).toBe(0)
 expect(body.levels[0]!.endpointContourErrorUpper![1]).toBeGreaterThan(0)
 expect(body.sections[0]).toEqual(body.sections.at(-1))
 expect(body.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,closed:true,filledCapErrorUpper:null})
})

it('charges explicit direct circle repair to original source and proves saved profile G2',()=>{
 const request:DirectMiterOwnedSourceRequest={...source(rings([1,0,0]),[[0,0,0],[10,0,0],[13,9,4e-12],[-2,11,1e-12],[1,5,-2e-12]]),normal:[0,0,1],closed:true,miterLimit:4,
  circleCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:1000000}}
 const raw=assertOwner({...request,circleCorrection:undefined}),body=assertOwner(request)
 expect(body.sourceSections).toEqual(raw.sourceSections)
 expect(body.sectionCorrection!.wallDisplacementUpper).toBeGreaterThan(0)
 expect(body.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(inspectMiterProfileSmoothness(body.model,[])).toMatchObject({profile:{exactG1G2Certified:true},stationContinuity:'C0',fullBoundarySmoothnessCertified:false})
 expect(()=>constructMiterOwnedBoundary({...request,circleCorrection:{...request.circleCorrection!,maxWork:0}})).toThrow(/circle section correction unproved/)
})
