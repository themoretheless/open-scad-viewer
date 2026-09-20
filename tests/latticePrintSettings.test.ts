import {describe,it,expect} from 'vitest'
import {fitLatticeToPrint,printBridgeWarning} from '../src/services/latticePrintSettings'
import type {LighteningOptions} from '../src/services/solidLightening'
const o:LighteningOptions={pattern:'web',axis:'x',cell:8,rib:1.35,rim:2,bottom:.1,top:.7,seed:42,jitter:.7,lineWidth:.45,perimeters:3}
const p={nozzle:.6,layer:.25,lines:4,skinLayers:4,maxBridge:5,openTop:true}
describe('print geometry fitting',()=>{
 it.each(['grid','web','isogrid','spatial','bcc','octet'] as const)('limits the nominal %s opening without thickening the fitted rib',pattern=>{
  const input={...o,pattern,cell:20},before=structuredClone(input)
  const fitted=fitLatticeToPrint(input,p),limited=fitLatticeToPrint(input,p,true)
  expect(limited.rib).toBe(fitted.rib)
  expect(limited.cell).toBeCloseTo(7.7,12)
  expect(limited.cell-limited.rib).toBeLessThanOrEqual(p.maxBridge)
  expect(limited.cell).toBeGreaterThanOrEqual(2*limited.rib+limited.lineWidth)
  expect(printBridgeWarning(limited,p)).toBe(false)
  expect(fitLatticeToPrint(limited,p,true)).toEqual(limited)
  expect(input).toEqual(before)
 })
 it('refuses incompatible opening limits atomically instead of making the opening larger',()=>{
  const input={...o,cell:20},before=structuredClone(input)
  expect(()=>fitLatticeToPrint(input,{...p,maxBridge:1},true)).toThrow('incompatible')
  expect(input).toEqual(before)
  expect(fitLatticeToPrint(input,{...p,maxBridge:1}).cell).toBe(20)
  expect(fitLatticeToPrint(input,p,true).cell).toBeCloseTo(7.7,12)
 })
 it('preserves an already fitting cell and rejects unrepresentable extrusion widths',()=>{
  const input={...o,cell:7}
  expect(fitLatticeToPrint(input,p,true)).toEqual(fitLatticeToPrint(input,p))
  for(const nozzle of [1e-20,Number.MAX_VALUE])expect(()=>fitLatticeToPrint(o,{...p,nozzle,layer:nozzle},true)).toThrow('numeric range')
 })
 it.each(['web','isogrid'] as const)('fits %s walls and floor, opens top and preserves source options',(pattern)=>{const r=fitLatticeToPrint({...o,pattern},p);expect(r.rib).toBe(2.7);expect(r.bottom).toBe(1);expect(r.top).toBe(0);expect(r.axis).toBe('z');expect(o.axis).toBe('x');expect(fitLatticeToPrint(r,p)).toEqual(r)})
 it.each(['spatial','bcc','octet'] as const)('resolves %s skin and sampling without removing diagonals',(pattern)=>{const r=fitLatticeToPrint({...o,pattern,skin:1,step:2,diagonals:true},p);expect(r.skin).toBe(2.7);expect(r.step).toBeLessThanOrEqual(r.rib/3);expect(r.diagonals).toBe(true);expect(r.axis).toBe('x');expect(r.bottom).toBe(o.bottom);expect(printBridgeWarning(r,p)).toBe(true)})
 it('rejects impossible layer heights and keeps optional shell absent',()=>{expect(()=>fitLatticeToPrint(o,{...p,layer:1})).toThrow();expect(fitLatticeToPrint({...o,pattern:'spatial',skin:0},p).skin).toBe(0)})
 it('is deterministic and rejects non-finite or out-of-range print settings',()=>{
  expect(fitLatticeToPrint(o,p)).toEqual(fitLatticeToPrint(o,p))
  for(const bad of [{...p,nozzle:NaN},{...p,nozzle:0},{...p,layer:0},{...p,layer:Infinity},{...p,lines:0},{...p,lines:9},{...p,lines:1.5},{...p,skinLayers:-1},{...p,skinLayers:.5},{...p,maxBridge:0},{...p,maxBridge:NaN}])expect(()=>fitLatticeToPrint(o,bad)).toThrow()
  expect(printBridgeWarning(o,{...p,maxBridge:10})).toBe(false)
  expect(()=>printBridgeWarning(o,{...p,maxBridge:NaN})).toThrow()
 })
})
