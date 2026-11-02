import {describe,it,expect} from 'vitest'
import {isSurfaceIsoTargetReport} from '../src/services/surfaceIsoTargetProtocol'
function report():any{return{schema:'nurbs-surface-iso-target-window/1',status:'present',axis:0,parameter:.5,sourceIsoParameterCoordinates:'normalized-original-transverse-domain',parameterCoordinates:['source-natural-t','target-natural-u','target-natural-v'],window:[[0,1],[-2,2],[10,14]],maxCells:100,cells:10,roots:[{method:'original-curve-surface-krawczyk',spans:[3,1,1],existenceDomain:[[.1,.4],[-1,1],[11,13]],parameterBox:[[.24,.26],[-.01,.01],[12,12.1]],positionBounds:[[-.01,.01],[-.01,.01],[1,1.1]],crossingSign:1}],presenceProven:true,windowAbsenceProven:false,windowCoverageComplete:true,unresolved:[],allSurfaceBranchesEnumerated:false,trimAdmission:false,bodyAdmission:false}}
describe('original iso target window protocol',()=>{
 it('accepts natural source and independent target domains',()=>expect(isSurfaceIsoTargetReport(report())).toBe(true))
 it('accepts incomplete presence with explicit remaining domains',()=>{const r=report();r.windowCoverageComplete=false;r.unresolved=[r.window];expect(isSurfaceIsoTargetReport(r)).toBe(true)})
 it('accepts absence scoped to the complete window',()=>{const r=report();Object.assign(r,{status:'absent-in-window',presenceProven:false,windowAbsenceProven:true,roots:[]});expect(isSurfaceIsoTargetReport(r)).toBe(true)})
 it.each(['allSurfaceBranchesEnumerated','trimAdmission','bodyAdmission'])('rejects forged %s',key=>{const r=report();r[key]=true;expect(isSurfaceIsoTargetReport(r)).toBe(false)})
 it('rejects a root outside its existence proof domain',()=>{const r=report();r.roots[0].parameterBox[0]=[.5,.6];expect(isSurfaceIsoTargetReport(r)).toBe(false)})
 it('rejects an existence proof outside the requested window',()=>{const r=report();r.roots[0].existenceDomain[2]=[9,15];expect(isSurfaceIsoTargetReport(r)).toBe(false)})
 it('rejects unresolved evidence outside its window',()=>{const r=report();r.windowCoverageComplete=false;r.unresolved=[[[0,1],[-3,3],[10,14]]];expect(isSurfaceIsoTargetReport(r)).toBe(false)})
 it('rejects completeness with unresolved domains',()=>{const r=report();r.unresolved=[r.window];expect(isSurfaceIsoTargetReport(r)).toBe(false)})
 it('rejects a coordinate-system swap',()=>{const r=report();r.parameterCoordinates.reverse();expect(isSurfaceIsoTargetReport(r)).toBe(false)})
 it.each([0,101,NaN])('rejects invalid charged cells %s',cells=>{const r=report();r.cells=cells;expect(isSurfaceIsoTargetReport(r)).toBe(false)})
 it('rejects nonfinite world evidence',()=>{const r=report();r.roots[0].positionBounds[0][0]=NaN;expect(isSurfaceIsoTargetReport(r)).toBe(false)})
 it('rejects sparse parameter evidence',()=>{const r=report();r.window=Array(3);expect(isSurfaceIsoTargetReport(r)).toBe(false)})
 it('rejects an unproved crossing orientation',()=>{const r=report();r.roots[0].crossingSign=0;expect(isSurfaceIsoTargetReport(r)).toBe(false)})
})
