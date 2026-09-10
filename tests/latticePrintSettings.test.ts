import {describe,it,expect} from 'vitest'
import {fitLatticeToPrint,printBridgeWarning} from '../src/services/latticePrintSettings'
import type {LighteningOptions} from '../src/services/solidLightening'
const o:LighteningOptions={pattern:'web',axis:'x',cell:8,rib:1.35,rim:2,bottom:.1,top:.7,seed:42,jitter:.7,lineWidth:.45,perimeters:3}
const p={nozzle:.6,layer:.25,lines:4,skinLayers:4,maxBridge:5,openTop:true}
describe('print geometry fitting',()=>{
 it('fits walls and floor, opens top and preserves source options',()=>{const r=fitLatticeToPrint(o,p);expect(r.rib).toBe(2.7);expect(r.bottom).toBe(1);expect(r.top).toBe(0);expect(r.axis).toBe('z');expect(o.axis).toBe('x');expect(fitLatticeToPrint(r,p)).toEqual(r)})
 it('resolves spatial skin and sampling without removing diagonals',()=>{const r=fitLatticeToPrint({...o,pattern:'spatial',skin:1,step:2,diagonals:true},p);expect(r.skin).toBe(2.7);expect(r.step).toBeLessThanOrEqual(r.rib/3);expect(r.diagonals).toBe(true);expect(printBridgeWarning(r,p)).toBe(true)})
 it('fits octet and BCC like other spatial lattices',()=>{for(const pattern of ['octet','bcc'] as const){const r=fitLatticeToPrint({...o,pattern,skin:1,step:2,wallDepth:3.6},p);expect(r.rib).toBe(2.7);expect(r.step).toBeLessThanOrEqual(r.wallDepth!/2);expect(r.skin).toBe(2.7)}})
 it('rejects impossible layer heights and keeps optional shell absent',()=>{expect(()=>fitLatticeToPrint(o,{...p,layer:1})).toThrow();expect(fitLatticeToPrint({...o,pattern:'spatial',skin:0},p).skin).toBe(0)})
})
