import { describe, expect, it } from 'vitest'
import { evaluateSafeMathExpression, parseCadQuantity } from '../src/services/cadQuantity'
describe('CAD quantities through WASM',()=>{
  it.each([['2 cm','length',20],['1 in','length',25.4],['-1,5mm','length',-1.5],['1e-3 m','length',1],['3.141592653589793 rad','angle',180],['90°','angle',90]] as const)('normalizes %s', (text,kind,value)=>{
    const result=parseCadQuantity(text,kind);expect(result.valid).toBe(true)
    if(result.valid)expect(result.value).toBeCloseTo(value,10)
  })
  it.each([['2 deg','length'],['2 cm','angle'],['2mm','scalar'],['','length'],['NaN','length'],['1e999','length'],['12mm junk','length']] as const)('rejects %s for %s',(text,kind)=>expect(parseCadQuantity(text,kind).valid).toBe(false))
  it('checks bounds after converting units',()=>{
    expect(parseCadQuantity('1 cm','length',0,5)).toEqual({valid:false,reason:'range'})
    expect(parseCadQuantity('0.1 cm','length',0,5)).toEqual({valid:true,value:1})
  })
  it('evaluates safe inline mathematical expressions with units and functions',()=>{
    expect(evaluateSafeMathExpression('10 + 5 * 2')).toBe(20)
    expect(evaluateSafeMathExpression('(10 + 5) / 3')).toBe(5)
    expect(evaluateSafeMathExpression('sqrt(16) + 2^3')).toBe(12)
    expect(parseCadQuantity('(10 + 5) / 2 mm','length')).toEqual({valid:true,value:7.5})
    expect(parseCadQuantity('25.4 / 2','scalar')).toEqual({valid:true,value:12.7})
    expect(parseCadQuantity('pi / 2 rad','angle')).toEqual({valid:true,value:90})
  })
})
