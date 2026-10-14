import {expect,it} from 'vitest'
import {certifiedSweepBoundaryErrorUpper,filledMiterCapErrorUpper} from '../src/services/nurbsFilledCapError'
const premises={idealCapDomainsCertified:true,retainedCapRegionsExact:true,projectionNormalDots:[[1,1],[-2,-1]] as [[number,number],[number,number]],endpointContourErrorUpper:[.01,.02] as [number,number],correctionDisplacementUpper:.25}
it('bounds both filled caps across reflected plane projection and correction',()=>{
 const r=filledMiterCapErrorUpper(premises)!
 expect(r[0]).toBeGreaterThanOrEqual(Math.SQRT2*(.01+.25))
 expect(r[1]).toBeGreaterThanOrEqual(Math.SQRT2*(.02+.25))
 expect(r[0]).toBeLessThan(.368)
 expect(filledMiterCapErrorUpper({...premises,endpointContourErrorUpper:[0,0],correctionDisplacementUpper:0})).toEqual([0,0])
})
it('uses the sharper transfer only for endpoints with exact parallelism',()=>{
 const general=filledMiterCapErrorUpper(premises)!
 const mixed=filledMiterCapErrorUpper({...premises,parallelPlanesCertified:[true,false]})!
 expect(mixed[0]).toBeGreaterThanOrEqual(.26)
 expect(mixed[0]).toBeLessThan(.260001)
 expect(mixed[1]).toBe(general[1])
 expect(filledMiterCapErrorUpper({...premises,parallelPlanesCertified:null})).toEqual(general)
 expect(filledMiterCapErrorUpper({...premises,parallelPlanesCertified:[true,true],idealCapDomainsCertified:false})).toBeNull()
})
it('refuses missing ownership, region, projection, correction and overflow without partial bounds',()=>{
 for(const change of [{idealCapDomainsCertified:false},{retainedCapRegionsExact:false},{projectionNormalDots:null},{endpointContourErrorUpper:null},{correctionDisplacementUpper:null},{correctionDisplacementUpper:-1},{endpointContourErrorUpper:[0,Number.MAX_VALUE] as [number,number]},{projectionNormalDots:[[1,1],[-1,1]] as [[number,number],[number,number]]}])expect(filledMiterCapErrorUpper({...premises,...change})).toBeNull()
})

it('bounds the union of wall/cap sets and refuses incomplete open boundaries',()=>{
 expect(certifiedSweepBoundaryErrorUpper(.1,[.2,.3],false)).toBe(.3)
 expect(certifiedSweepBoundaryErrorUpper(.4,[.2,.3],false)).toBe(.4)
 expect(certifiedSweepBoundaryErrorUpper(.1,null,true)).toBe(.1)
 expect(certifiedSweepBoundaryErrorUpper(.1,null,false)).toBeNull()
 expect(certifiedSweepBoundaryErrorUpper(null,[.2,.3],false)).toBeNull()
 expect(certifiedSweepBoundaryErrorUpper(.1,[.2,NaN],false)).toBeNull()
})

it('adds decomposition before transferring either filled region and refuses a missing pair',()=>{
 const result=filledMiterCapErrorUpper({...premises,decompositionErrorUpper:[.03,.04],parallelPlanesCertified:[true,true]})!
 expect(result[0]).toBeGreaterThanOrEqual(.29)
 expect(result[1]).toBeGreaterThanOrEqual(.31)
 expect(filledMiterCapErrorUpper({...premises,decompositionErrorUpper:null})).toBeNull()
 expect(filledMiterCapErrorUpper({...premises,decompositionErrorUpper:[0,NaN]})).toBeNull()
})
