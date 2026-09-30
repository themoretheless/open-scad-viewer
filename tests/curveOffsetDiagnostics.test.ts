import {expect,it} from 'vitest'
import {validCurveOffsetDiagnostics} from '../src/services/curveOffsetDiagnostics'
const base=()=>({scope:'represented-offset-chain',method:'outward-line-pair-interval/1',originalOffsetTopologyCertified:false,crossings:[[0,2]],contacts:[],uncertain:[],degenerate:[],checks:3,totalPairs:3,enumerationComplete:true,predicatesComplete:true,complete:true,simple:false})
it('accepts a completed crossing report and a bounded partial enumeration',()=>{
 expect(validCurveOffsetDiagnostics(base(),3)).toBe(true)
 expect(validCurveOffsetDiagnostics({...base(),crossings:[],checks:1,enumerationComplete:false,complete:false},3)).toBe(true)
})
it('rejects false completeness, unknown indices, duplicate pairs and topology claims',()=>{
 for(const patch of [{simple:true},{checks:4},{totalPairs:4},{crossings:[[0,3]]},{contacts:[[0,2]]},{uncertain:[[0,1]]},{originalOffsetTopologyCertified:true}])
  expect(validCurveOffsetDiagnostics({...base(),...patch},3)).toBe(false)
})
