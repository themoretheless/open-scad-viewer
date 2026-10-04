import {expect,it} from 'vitest'
import {mainSolidResult} from '../src/services/mainSolidProtocol'
function response(){return {document:{version:1,sketches:[],bodies:[],curves:[]},report:{accepted:true,closed:false,errorUpperMm:.001,toleranceMm:.01,wholeCurve:true,method:'outward-rational-jets-chord-bound/1',cells:[{domain:[0,1],errorUpperMm:.001}],chainDiagnostics:{scope:'represented-offset-chain',method:'outward-line-pair-interval/1',originalOffsetTopologyCertified:false,crossings:[],contacts:[],uncertain:[],degenerate:[],checks:0,totalPairs:0,enumerationComplete:true,predicatesComplete:true,complete:true,simple:true},offsetRegularityCertified:false,regionTopologyCertified:false}}}
it('admits the bounded report while preserving its topology limitation',()=>{
 expect(mainSolidResult({kind:'curveOffset'},response())).toBe(true)
 for(const mutate of [
  (v:ReturnType<typeof response>)=>{v.report.errorUpperMm=.02},
  (v:ReturnType<typeof response>)=>{v.report.cells[0]!.domain=[1,0]},
  (v:ReturnType<typeof response>)=>{v.report.regionTopologyCertified=true},
  (v:ReturnType<typeof response>)=>{v.report.chainDiagnostics.originalOffsetTopologyCertified=true},
 ]){const v=response();mutate(v);expect(mainSolidResult({kind:'curveOffset'},v)).toBe(false)}
})
