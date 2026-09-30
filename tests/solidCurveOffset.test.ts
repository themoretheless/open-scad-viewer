import {beforeEach,describe,expect,it,vi} from 'vitest'
import type {DirectDocument} from '../src/services/directModeling'
const {native}=vi.hoisted(()=>({native:vi.fn()}))
vi.mock('../src/services/geometry/nurbs',()=>({callNurbsRust:native}))
import {offsetSolidCurve} from '../src/services/solidCurveOffset'
const curve={degree:1,knots:[0,0,1,1],weights:[1,1],controlPoints:[[0,0],[10,0]]}
const options={id:'source',createdId:'offset',distance:2,toleranceMm:.01,maxCells:512,maxPairs:1000}
function source(z?:number):DirectDocument{return {version:1,sketches:[],bodies:[],curves:[{id:'source',name:'Profile',group:'group',curve:{...structuredClone(curve),controlPoints:curve.controlPoints.map(p=>z===undefined?[...p]:[...p,z])}}]}}
function result(){return {curves:[{...structuredClone(curve),controlPoints:[[0,2],[10,2]]}],report:{accepted:true,closed:false,errorUpperMm:.001,toleranceMm:.01,wholeCurve:true,method:'outward-rational-jets-chord-bound/1',cells:[{domain:[0,1],errorUpperMm:.001}],chainDiagnostics:{scope:'represented-offset-chain',method:'outward-line-pair-interval/1',originalOffsetTopologyCertified:false,crossings:[],contacts:[],uncertain:[],degenerate:[],checks:0,totalPairs:0,enumerationComplete:true,predicatesComplete:true,complete:true,simple:true},offsetRegularityCertified:false,regionTopologyCertified:false}}}
beforeEach(()=>native.mockReset().mockReturnValue(result()))
describe('bounded NURBS offset document',()=>{
 it('preserves the source, group and Z while adding independent editable geometry',()=>{
  const d=source(7),before=structuredClone(d),r=offsetSolidCurve(d,options)
  expect(d).toEqual(before);expect(r.document.curves![0]).toEqual(before.curves![0])
  expect(r.document.curves![1]).toMatchObject({id:'offset',group:'group',curve:{controlPoints:[[0,2,7],[10,2,7]]}})
  expect(native.mock.calls[0]![1].curve.controlPoints).toEqual([[0,0],[10,0]])
  r.document.curves![0]!.curve.controlPoints[0]![0]=99;expect(d).toEqual(before)
 })
 it('rejects a nonplanar source before calculation',()=>{
  const d=source(7);d.curves![0]!.curve.controlPoints[1]![2]=8
  expect(()=>offsetSolidCurve(d,options)).toThrow('XY plane');expect(native).not.toHaveBeenCalled()
 })
 it('rejects output identity collisions without changing the document',()=>{
  const d=source(),before=structuredClone(d)
  expect(()=>offsetSolidCurve(d,{...options,createdId:'source'})).toThrow('unique identity');expect(d).toEqual(before)
 })
 it('refuses results exceeding tolerance',()=>{
  const invalid=result();invalid.report.errorUpperMm=.02;native.mockReturnValue(invalid)
  expect(()=>offsetSolidCurve(source(),options)).toThrow('certificate')
 })
})

it('refuses an offset that could not survive the document collection limit',()=>{
 const d=source(),item=d.curves![0]!
 for(let i=0;i<127;i++)d.curves!.push({...structuredClone(item),id:'existing-'+i})
 const before=structuredClone(d)
 expect(()=>offsetSolidCurve(d,options)).toThrow('128 NURBS objects');expect(d).toEqual(before)
})
it('refuses a positional report attached to mismatched retained parameter cells',()=>{
 const response=result();response.report.cells[0]!.domain=[0,.5];native.mockReturnValue(response)
 expect(()=>offsetSolidCurve(source(),options)).toThrow('cells do not match')
})

it('retains explicit bevel source roles and crossing diagnostics without a trimmed-region claim',()=>{
 const d=source();d.curves![0]!.curve={degree:1,knots:[0,0,.5,1,1],weights:[1,1,1],controlPoints:[[0,0],[10,0],[10,10]]}
 const response=result() as any
 response.curves=[{degree:1,knots:[0,0,1,2,3,3],weights:[1,1,1,1],controlPoints:[[0,2],[10,2],[8,0],[8,10]]}]
 response.report={...response.report,wholeCurve:false,wholeWire:true,regionTrimmed:false,method:'outward-source-offset-bevel-wire/1',cells:[
  {domain:[0,1],errorUpperMm:.001,source:{kind:'source-offset',domain:[0,.5]}},
  {domain:[1,2],errorUpperMm:.001,source:{kind:'bevel',sourceKnot:.5}},
  {domain:[2,3],errorUpperMm:.001,source:{kind:'source-offset',domain:[.5,1]}},
 ],chainDiagnostics:{...response.report.chainDiagnostics,crossings:[[0,2]],checks:3,totalPairs:3,simple:false}}
 native.mockReturnValue(response)
 const before=structuredClone(d),r=offsetSolidCurve(d,{...options,join:'bevel'})
 expect(native.mock.calls[0]![0]).toBe('curve_offset_bevel_wire');expect(r.report.regionTrimmed).toBe(false)
 expect(r.report.cells[1]!.source).toEqual({kind:'bevel',sourceKnot:.5});expect(d).toEqual(before)
 response.report.cells[2].source.domain=[.6,1]
 expect(()=>offsetSolidCurve(d,{...options,join:'bevel'})).toThrow('source roles')
 response.report.cells[2].source.domain=[.5,1]
 delete response.report.cells[1].source
 expect(()=>offsetSolidCurve(d,{...options,join:'bevel'})).toThrow('source roles')
})
