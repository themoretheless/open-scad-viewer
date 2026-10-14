import {expect,it} from 'vitest'
import {callNurbsRust} from '../src/services/geometry/nurbs'
const premises={idealCapDomainsCertified:true,retainedCapRegionsExact:true,projectionNormalDots:[[1,1],[-1,-1]],endpointContourErrorUpper:[0,0],correctionDisplacementUpper:.125,decompositionErrorUpper:[0,0],parallelPlanesCertified:[true,false]}
it('keeps native conditional cap bounds distinct from continuous admission',()=>{
 const report=callNurbsRust<{capErrorUpper:[number,number]|null;continuousBound:boolean}>('sweep_filled_cap_error_upper',premises)
 expect(report.continuousBound).toBe(false)
 expect(report.capErrorUpper![0]).toBe(.125)
 expect(report.capErrorUpper![1]).toBeGreaterThan(.125)
 for(const change of [{idealCapDomainsCertified:false},{retainedCapRegionsExact:false},{decompositionErrorUpper:null},{projectionNormalDots:[[1,1],[-1,1]]},{endpointContourErrorUpper:[0,Number.MAX_VALUE]}]){
  expect(callNurbsRust('sweep_filled_cap_error_upper',{...premises,...change})).toMatchObject({capErrorUpper:null,continuousBound:false})
 }
 expect(()=>callNurbsRust('sweep_filled_cap_error_upper',{...premises,endpointContourErrorUpper:[0]})).toThrow()
})
it('requires cap coverage for open native boundary unions',()=>{
 expect(callNurbsRust('sweep_boundary_error_upper',{wall:1,caps:[2,3],closed:false})).toEqual({boundaryErrorUpper:3,continuousBound:false})
 expect(callNurbsRust('sweep_boundary_error_upper',{wall:1,caps:null,closed:false})).toEqual({boundaryErrorUpper:null,continuousBound:false})
 expect(callNurbsRust('sweep_boundary_error_upper',{wall:1,caps:null,closed:true})).toEqual({boundaryErrorUpper:1,continuousBound:false})
})
