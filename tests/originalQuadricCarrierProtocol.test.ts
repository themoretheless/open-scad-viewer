import {describe,it,expect} from 'vitest'
import {isOriginalQuadricCarrierReport as valid} from '../src/services/originalQuadricCarrierProtocol'
import {quadricReport} from './fixtures/quadricCarrier'
describe('original quadric carrier evidence',()=>{
 it('accepts complete carrier roots without finite surface admission',()=>expect(valid(quadricReport())).toBe(true))
 it('retains degree-50 residual multiplicity',()=>{const r=quadricReport();r.roots=r.roots.slice(0,1);r.roots[0].multiplicities=[50];r.scalarProofs[0].normalizedOriginalSpanRoots=[{interval:[0,0],multiplicity:50}];expect(valid(r)).toBe(true)})
 it('retains distinct one-sided root orders at a shared knot',()=>{const r=quadricReport();r.roots=r.roots.slice(0,1);r.roots[0].sourceSpans=[1,2];r.roots[0].multiplicities=[1,2];const p=structuredClone(r.scalarProofs[0]);p.span=2;p.normalizedOriginalSpanRoots=[{interval:[0,0],multiplicity:2}];r.scalarProofs.push(p);r.spans=2;r.exactWork=60;expect(valid(r)).toBe(true)})
 it('accepts complete absence on the infinite carrier',()=>{const r=quadricReport();Object.assign(r,{status:'absent-on-carrier',presenceProven:false,carrierAbsenceProven:true,roots:[]});expect(valid(r)).toBe(true)})
 it('accepts zero work with explicit unresolved span',()=>{const r=quadricReport();Object.assign(r,{status:'unresolved',presenceProven:false,curveCarrierCoverageComplete:false,roots:[],scalarProofs:[],unresolvedSpans:[1],spans:0,exactWork:0,maxExactWork:0});expect(valid(r)).toBe(true)})
 it('accepts a coincident span as missing isolated-root coverage',()=>{const r=quadricReport();Object.assign(r,{status:'unresolved',presenceProven:false,curveCarrierCoverageComplete:false,roots:[],unresolvedSpans:[1]});Object.assign(r.scalarProofs[0],{status:'coincident',normalizedOriginalSpanRoots:[]});expect(valid(r)).toBe(true)})
 it.each(['proofContextSerialized','finiteSurfaceUvAdmission','allSurfaceBranchesEnumerated','trimAdmission','bodyAdmission'])('rejects forged %s',key=>{const r=quadricReport();r[key]=true;expect(valid(r)).toBe(false)})
 it('rejects an invented crossing sign',()=>{const r=quadricReport();r.roots[0].crossingSign=1;expect(valid(r)).toBe(false)})
 it('rejects unmatched multiplicities and source spans',()=>{for(const change of [(r:any)=>r.roots[0].multiplicities=[2],(r:any)=>r.roots[0].sourceSpans=[2]]){const r=quadricReport();change(r);expect(valid(r)).toBe(false)}})
 it('rejects multiplicity above scalar degree 50',()=>{const r=quadricReport();r.roots[0].multiplicities=[51];r.scalarProofs[0].normalizedOriginalSpanRoots[0].multiplicity=51;expect(valid(r)).toBe(false)})
 it('rejects missing or mismatched work accounting',()=>{const r=quadricReport();r.exactWork=31;expect(valid(r)).toBe(false)})
 it('rejects a complete result with unresolved spans',()=>{const r=quadricReport();r.unresolvedSpans=[1];expect(valid(r)).toBe(false)})
 it('rejects an indeterminate span without missing coverage',()=>{const r=quadricReport();Object.assign(r.scalarProofs[0],{status:'indeterminate',reason:'ResourceLimit',normalizedOriginalSpanRoots:[]});expect(valid(r)).toBe(false)})
 it('rejects unsupported reason strings',()=>{const r=quadricReport();r.roots=[];Object.assign(r,{status:'unresolved',presenceProven:false,curveCarrierCoverageComplete:false,unresolvedSpans:[1]});Object.assign(r.scalarProofs[0],{status:'indeterminate',reason:'success',normalizedOriginalSpanRoots:[]});expect(valid(r)).toBe(false)})
 it('rejects normalized roots outside the original span',()=>{const r=quadricReport();r.scalarProofs[0].normalizedOriginalSpanRoots[0].interval=[-1,2];expect(valid(r)).toBe(false)})
 it('rejects unqueried complete absence',()=>{const r=quadricReport();Object.assign(r,{status:'absent-on-carrier',presenceProven:false,carrierAbsenceProven:true,roots:[],scalarProofs:[],spans:0,exactWork:0});expect(valid(r)).toBe(false)})
 it('rejects nonfinite and sparse bounds',()=>{for(const value of [Infinity,undefined]){const r=quadricReport();r.roots[0].positionBounds[0][0]=value;expect(valid(r)).toBe(false)}})
 it('rejects duplicate scalar spans and duplicate side references',()=>{const r=quadricReport();r.scalarProofs.push(structuredClone(r.scalarProofs[0]));r.spans=2;r.exactWork=60;expect(valid(r)).toBe(false);const q=quadricReport();q.roots[0].sourceSpans=[1,1];q.roots[0].multiplicities=[1,1];expect(valid(q)).toBe(false)})
})
