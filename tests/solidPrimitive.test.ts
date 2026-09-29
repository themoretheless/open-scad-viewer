import {expect,it} from 'vitest'
import {addSolidPrimitive,type SolidPrimitiveKind,type SolidPrimitiveOptions} from '../src/services/solidPrimitive'
import {emptyDirectDocument} from '../src/services/directModeling'
import {analyzeNurbsBrep} from '../src/services/geometry/brep'
const options=(kind:SolidPrimitiveKind):SolidPrimitiveOptions=>({kind,id:kind,name:kind,size:20,topRadius:5,innerRadius:7,round:'exact',segments:4,group:'Parts'})
it.each([
 ['box',8000],['wedge',4000],['cylinder',2000*Math.PI],['cone',2000*Math.PI/3],
 ['frustum',20*Math.PI*(100+50+25)/3],['tube',20*Math.PI*(100-49)],['sphere',4000*Math.PI/3],['torus',2*Math.PI*Math.PI*10*25],
] as const)('authors %s with the requested dimensions and leaves the input intact',(kind,volume)=>{
 const source=emptyDirectDocument(),before=structuredClone(source),result=addSolidPrimitive(source,options(kind)),body=result.bodies[0]
 expect(source).toEqual(before);expect(body.id).toBe(kind);expect(body.group).toBe('Parts')
 expect(analyzeNurbsBrep(body.brep!).signedVolumeMm3).toBeCloseTo(volume,5)
 expect(body.mesh.positions.byteLength).toBeGreaterThan(0)
})
it('preserves faceted round modes and rejects invalid dimensions atomically',()=>{
 const source=emptyDirectDocument()
 for(const kind of ['cone','sphere','cylinder'] as const){
  const body=addSolidPrimitive(source,{...options(kind),round:'faceted'}).bodies[0]
  if(kind==='cone')expect(body.brep).toBeUndefined()
  else expect(body.brep?.faces.length).toBeGreaterThan(6)
 }
 expect(()=>addSolidPrimitive(source,{...options('box'),size:NaN})).toThrow('Size')
 expect(()=>addSolidPrimitive(source,{...options('tube'),innerRadius:11})).toThrow()
 expect(source.bodies).toHaveLength(0)
})
