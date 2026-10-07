import {expect,it,vi} from 'vitest'
const native=vi.hoisted(()=>vi.fn())
vi.mock('../src/services/geometry/nurbs',()=>({callNurbsRust:native}))
import {offsetTrimmedSolidCurve} from '../src/services/solidTrimmedCurveOffset'
const curve={degree:1,knots:[0,0,1,2,3,3],weights:[1,1,1,1],controlPoints:[[0,0],[2,0],[0,2],[0,0]]}
const source={version:1 as const,bodies:[],sketches:[],curves:[{id:'a',name:'A',curve}]}
const options={id:'a',createdId:'new',distance:1,toleranceMm:0.01,maxCells:100,maxPairs:100,maxWitnessChecks:100,fillRule:'nonzero' as const,intersectionToleranceMm:0.001}
const report={accepted:true,method:'represented-bevel-offset-fill/1',fillRule:'nonzero',regionTrimmed:true,topologyScope:'represented-reconstructed-chord-graph',regionTopologyCertified:false,originalOffsetTopologyCertified:false,sourceWireErrorUpperMm:0.005,intersectionConstructionErrorUpperMm:0.0001}
it('retains independent closed loops and leaves the source untouched',()=>{
 native.mockReturnValue({loops:[[curve],[curve]],report})
 const result=offsetTrimmedSolidCurve(source,options)
 expect(result.loopIds).toEqual([['new:0:0'],['new:1:0']]);expect(result.document.curves).toHaveLength(3)
 expect(result.document.curves![1]!.offsetRegion).toMatchObject({loopId:'new:0',part:0,parts:1,scope:'at-construction'});expect(source.curves).toHaveLength(1);expect(result.report.originalOffsetTopologyCertified).toBe(false)
})
it('refuses broken closure, excessive error and duplicate identities before mutation',()=>{
 for(const result of [
  {loops:[[{...curve,controlPoints:[[0,0],[2,0],[0,2],[1,0]]}]],report},
  {loops:[[curve]],report:{...report,intersectionConstructionErrorUpperMm:1}},
  {loops:[[curve]],report:{...report,originalOffsetTopologyCertified:true}},
 ]) {native.mockReturnValue(result);expect(()=>offsetTrimmedSolidCurve(source,options)).toThrow()}
 native.mockReturnValue({loops:[[curve]],report})
 const existing={...source,curves:[...source.curves,{id:'new:0:0',name:'Existing',curve}]}
 expect(()=>offsetTrimmedSolidCurve(existing,options)).toThrow('identities')
 expect(existing.curves).toHaveLength(2)
})
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
it('worker admission checks the requested rule, bounds and loop identities',()=>{
 native.mockReturnValue({loops:[[curve]],report})
 const value=offsetTrimmedSolidCurve(source,options)
 const request=mainSolidExpectation({kind:'trimmedCurveOffset',document:source,options})
 expect(mainSolidResult(request,value)).toBe(true)
 expect(mainSolidResult(request,{...value,report:{...report,fillRule:'evenodd'}})).toBe(false)
 expect(mainSolidResult(request,{...value,loopIds:[['new:0:0'],['new:0:0']]})).toBe(false)
 expect(mainSolidResult(request,{...value,report:{...report,sourceWireErrorUpperMm:1}})).toBe(false)
})

it('worker admission refuses an open loop even with a valid curve definition',()=>{
 native.mockReturnValue({loops:[[curve]],report})
 const value=offsetTrimmedSolidCurve(source,options)
 const broken=structuredClone(value)
 broken.document.curves![1]!.curve.controlPoints[3]=[1,0]
 expect(mainSolidResult(mainSolidExpectation({kind:'trimmedCurveOffset',document:source,options}),broken)).toBe(false)
})

it('empty filled region gives an actionable result before changing the document',()=>{
 native.mockReturnValue({loops:[],report})
 expect(()=>offsetTrimmedSolidCurve(source,options)).toThrow('Reduce the offset')
 expect(source.curves).toHaveLength(1)
})

import {validCurveOffsetRegion} from '../src/services/curveOffsetRegion'
it('saved loop metadata rejects invalid part indices and false certification',()=>{
 const value={version:1,scope:'at-construction',sourceId:'source',loopId:'loop',part:0,parts:2,fillRule:'evenodd',originalOffsetTopologyCertified:false}
 expect(validCurveOffsetRegion(value)).toBe(true)
 expect(validCurveOffsetRegion({...value,part:2})).toBe(false)
 expect(validCurveOffsetRegion({...value,originalOffsetTopologyCertified:true})).toBe(false)
})

it('worker result identities must belong to the requested construction',()=>{
 native.mockReturnValue({loops:[[curve]],report})
 const value=offsetTrimmedSolidCurve(source,options)
 const wrong=structuredClone(value);wrong.document.curves![1]!.id='other:0:0';wrong.loopIds=[['other:0:0']]
 expect(mainSolidResult(mainSolidExpectation({kind:'trimmedCurveOffset',document:source,options}),wrong)).toBe(false)
})
