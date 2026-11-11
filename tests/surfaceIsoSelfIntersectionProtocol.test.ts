import {describe,it,expect} from 'vitest'
import {isSurfaceIsoAuditReport} from '../src/kernel/surfaceIsoSelfIntersectionProtocol'
function report():any{return{schema:'nurbs-surface-iso-self-intersection/1',status:'present',axis:0,parameter:.25,maxSpans:100,cells:4,presenceProven:true,isoAbsenceProven:false,surfaceAbsenceProven:false,isoCoverageComplete:false,allContactBranchesEnumerated:false,witnessCoordinates:'normalized-original-active-domain',crossings:[{method:'original-planar-krawczyk',parameterBoxes:[[[.2,.3],[.25,.25]],[[.7,.8],[.25,.25]]],positionBounds:[[0,0],[0,0],[1,1]]}],unresolvedCurveParameters:[[[0,1],[0,1]]]}}
describe('selected surface iso evidence',()=>{
 it('accepts an isolated contact and unresolved remaining curve domains',()=>expect(isSurfaceIsoAuditReport(report())).toBe(true))
 it('accepts a complete simple fiber without asserting whole-surface absence',()=>{const r=report();Object.assign(r,{status:'iso-simple',presenceProven:false,isoAbsenceProven:true,isoCoverageComplete:true,crossings:[],unresolvedCurveParameters:[]});expect(isSurfaceIsoAuditReport(r)).toBe(true)})
 it('accepts a V-axis certificate',()=>{const r=report();r.axis=1;for(const uv of r.crossings[0].parameterBoxes)uv.reverse();expect(isSurfaceIsoAuditReport(r)).toBe(true)})
 it.each(['surfaceAbsenceProven','allContactBranchesEnumerated'])('rejects unsupported %s',key=>{const r=report();r[key]=true;expect(isSurfaceIsoAuditReport(r)).toBe(false)})
 it('rejects a root on the wrong selected fiber',()=>{const r=report();r.crossings[0].parameterBoxes[1][1]=[.3,.3];expect(isSurfaceIsoAuditReport(r)).toBe(false)})
 it('rejects the trivial parameter diagonal',()=>{const r=report();r.crossings[0].parameterBoxes[1]=r.crossings[0].parameterBoxes[0];expect(isSurfaceIsoAuditReport(r)).toBe(false)})
 it('rejects coverage with unresolved cells',()=>{const r=report();r.isoCoverageComplete=true;expect(isSurfaceIsoAuditReport(r)).toBe(false)})
 it.each([NaN,Infinity,0,1])('rejects invalid selected parameter %s',parameter=>{const r=report();r.parameter=parameter;expect(isSurfaceIsoAuditReport(r)).toBe(false)})
 it('rejects sampled or unrelated proof methods',()=>{const r=report();r.crossings[0].method='sampled';expect(isSurfaceIsoAuditReport(r)).toBe(false)})
 it('rejects overspent resource evidence',()=>{const r=report();r.cells=101;expect(isSurfaceIsoAuditReport(r)).toBe(false)})
})
