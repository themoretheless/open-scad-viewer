import {expect,it,vi} from 'vitest'
const native=vi.hoisted(()=>vi.fn())
vi.mock('../src/services/geometry/nurbs',()=>({callNurbsRust:native}))
import {createG2RunoutSurface} from '../src/services/geometricG2'
import type {NurbsSurface} from '../src/services/nurbsSurface'
const surface:NurbsSurface={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]]}
const pc={degree:1,knots:[0,0,1,1],controlPoints:[[0,.1],[1,.1]],weights:[1,1]}
const options={sourceBoundary:'vMax' as const,sourceTrace:{at:.9,along:[0,1] as [number,number]},transverseUv:[0,1] as [number,number],gains:[.1,.1] as [number,number]}
function receipt(){return {patches:[surface],report:{method:'native-original-support-G2-runout/1',complete:true,withinTolerance:true,cells:12,exactWork:100,
 maxCells:100000,maxExactWork:1000000,tolerances:[1e-7,1e-5,1e-4],solidGeometryCertified:false,radiusLawCertified:false,interiorEmbeddingCertified:false,
 supportSeams:[0,1].map(source=>({patch:0,source,positionValidated:true,positionUpper:1e-8,inspection:{decision:'within-tolerance',regularityProven:true,cells:6,
  positionBounds:[0,1e-8],orientation:'same',branches:[]}})),internalSeams:[]}}}
// Transport/receipt tests only; geometry qualification is covered natively.
it('passes original-support inputs and preserves unresolved diagnostics',()=>{
 native.mockReturnValue(receipt())
 expect(createG2RunoutSurface(surface,surface,pc,options).report.withinTolerance).toBe(true)
 expect(native).toHaveBeenLastCalledWith('brep_g2_runout_surface',expect.objectContaining({source:surface,plane:surface,targetPcurve:pc,
  sourceTrace:options.sourceTrace,maxCells:100000,maxExactWork:1000000,reverseU:[false,false]}))
 const unresolved=receipt();unresolved.report.complete=false;unresolved.report.withinTolerance=false;unresolved.report.supportSeams=[]
 native.mockReturnValue(unresolved)
 expect(createG2RunoutSurface(surface,surface,pc,options).report).toBe(unresolved.report)
})
it('rejects forged body claims, over-budget work and missing or duplicated seam coverage',()=>{
 const changes=[(r:any)=>r.report.solidGeometryCertified=true,(r:any)=>r.report.radiusLawCertified=true,
  (r:any)=>r.report.cells=100001,(r:any)=>r.report.exactWork=-1,(r:any)=>r.report.supportSeams.pop(),
  (r:any)=>r.report.supportSeams[1].source=0,(r:any)=>r.report.supportSeams[0].positionValidated=false]
 for(const change of changes){const r=receipt();change(r);native.mockReturnValue(r);expect(()=>createG2RunoutSurface(surface,surface,pc,options)).toThrow()}
})
it('checks cancellation before dispatch and before exposing the result',()=>{
 const before=vi.fn(()=>{throw Error('cancelled')});native.mockClear()
 expect(()=>createG2RunoutSurface(surface,surface,pc,options,before)).toThrow('cancelled');expect(native).not.toHaveBeenCalled()
 native.mockReturnValue(receipt());let visits=0
 expect(()=>createG2RunoutSurface(surface,surface,pc,options,()=>{if(++visits===2)throw Error('cancelled')})).toThrow('cancelled')
})
