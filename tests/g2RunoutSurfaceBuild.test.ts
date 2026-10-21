import {expect,it,vi} from 'vitest'
const native=vi.hoisted(()=>vi.fn())
vi.mock('../src/services/geometry/nurbs',()=>({callNurbsRust:native}))
import {buildSolidSurface,type SolidSurfaceBuildOptions} from '../src/services/solidSurfaceConstruction'
import type {DirectDocument} from '../src/services/directModeling'
const surface={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]]}
function document():DirectDocument{return {version:1,bodies:[],sketches:[],surfaces:['a','b'].map(id=>({id,name:id,surface:structuredClone(surface),segmentsU:4,segmentsV:4})),curves:[{id:'c',name:'Boundary',curve:{degree:1,knots:[0,0,1,1],controlPoints:[[0,1,.1],[1,1,.1]],weights:[1,1]}}]}}
const options:SolidSurfaceBuildOptions={kind:'nurbs-runout',ids:['a','b','c'],id:'result',group:'group-a',reversed:[],sweep:{mode:'translation',normal:[0,0,1],sections:4,maxDeviation:.01},
 runout:{sourceBoundary:'vMax',sourcePosition:.9,tangentLengthMm:.1,flipSide:false,reverse:false}}
function receipt(){return {patches:[surface],report:{method:'native-original-support-G2-runout/1',complete:true,withinTolerance:true,cells:8,exactWork:928,maxCells:100000,maxExactWork:1000000,
 tolerances:[1e-7,1e-5,1e-4],solidGeometryCertified:false,radiusLawCertified:false,interiorEmbeddingCertified:false,internalSeams:[],
 supportSeams:[0,1].map(source=>({patch:0,source,positionValidated:true,positionUpper:1e-8,inspection:{decision:'within-tolerance',regularityProven:true,cells:4,branches:[],positionBounds:[0,1e-8],orientation:'same'}}))}}}
// Document and transport behavior; these receipts do not qualify fixture geometry.
it('delegates spatial input and physical sizes to Rust and appends an independent surface',()=>{
 const input=document(),before=structuredClone(input);native.mockReturnValue(receipt())
 const r=buildSolidSurface(input,options)
 expect(r.error).toBe('');expect(r.document?.surfaces).toHaveLength(3);expect(r.document?.surfaces?.slice(0,2)).toEqual(before.surfaces)
 expect(r.document?.curves).toEqual(before.curves);expect(r.document?.surfaces?.[2]).toMatchObject({id:'result',group:'group-a'})
 expect(input).toEqual(before)
 expect(native).toHaveBeenLastCalledWith('brep_g2_runout_surface',expect.objectContaining({targetCurve:input.curves![0].curve,sourcePosition:.9,tangentLengthMm:.1}))
})
it('keeps an unproven result diagnostic and refuses malformed or duplicate roles before native execution',()=>{
 const input=document(),r=receipt();r.report.complete=false;r.report.withinTolerance=false;r.report.supportSeams=[];native.mockReturnValue(r)
 expect(buildSolidSurface(input,options)).toMatchObject({document:null,error:'RUNOUT_G2_UNPROVEN',runoutReport:{withinTolerance:false}})
 native.mockClear();expect(buildSolidSurface(input,{...options,ids:['a','a','c']})).toMatchObject({document:null,error:'RUNOUT_INPUTS'});expect(native).not.toHaveBeenCalled()
 input.curves![0].curve.controlPoints.forEach(p=>p.pop());expect(buildSolidSurface(input,options).document).toBe(null);expect(native).not.toHaveBeenCalled()
})
