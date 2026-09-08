import { expect, it } from 'vitest'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { parseOpenSCAD } from '../src/services/openscadParser'
const source=`// @modelgraph-text/1
param radius = 20mm range 1mm..50mm
param height = 10mm range 1mm..30mm
expand = x => x + 1mm
body = circle(expand(radius)).extrude(height)
show body
`
it.each(['[2mm, 3mm, 4mm]', 'x: 2mm, z: 4mm', 'vector: [2, 3, 4]'])('moves geometry with the same relative displacement as translate: %s',args=>{
 const source=`// @modelgraph-text/1\nshow box([1,2,3]).move(${args})`
 expect(compileModelGraphText(source).source).toBe(compileModelGraphText(source.replace('.move(','.translate(')).source)
})
it('lowers parameters, units, closures and pipelines and preserves slider spans',()=>{
 const c=compileModelGraphText(source)
 expect(c.document.parameters[0]).toMatchObject({id:'radius',value:20,unit:'mm',min:1,max:50})
 const p=c.customizer[0]!
 expect(source.slice(p.valueStart,p.valueEnd)).toBe('20')
 expect(source.slice(p.valueEnd,p.valueEnd+2)).toBe('mm')
 expect(c.source).toContain('21')
})
it('builds compact source through the real parser',async()=>{
 const built=await parseOpenSCAD('// @modelgraph-text/1\nbody = box([2mm,3mm,4mm])\nshow body')
 expect(built.meshes.length).toBe(1)
 expect(built.meshes[0]!.provenance.every(p=>p.source===null)).toBe(true)
})
it('expands repeat with lexical index and short functions',()=>{
 const c=compileModelGraphText('// @modelgraph-text/1\nstep = x => x * 3mm\nparts = repeat(18, i => sphere(1mm).translate([step(i),0,0]))\nshow parts')
 expect(c.document.nodes.some(n=>n.op==='map')).toBe(true)
})
it.each(['param x = 2mm range 3mm..4mm\na = sphere(x)','a = unknown(2)','a = sphere(1mm).box([1,2,3])','a = sphere(1mm)\na = sphere(2mm)','a = sphere(1mm); @'])('rejects invalid source: %s',body=>{
 expect(()=>compileModelGraphText('// @modelgraph-text/1\n'+body)).toThrow()
})
it('builds the documented ring pattern with parameter editing',async()=>{
 const {readFileSync}=await import('node:fs')
 const text=readFileSync('examples/modelgraph-text/ring-pattern.scad','utf8')
 const compiled=compileModelGraphText(text)
 const p=compiled.customizer.find(p=>p.name==='height')!
 const edited=text.slice(0,p.valueStart)+'10'+text.slice(p.valueEnd)
 expect(compileModelGraphText(edited).document.parameters.find(p=>p.id==='height')!.value).toBe(10)
 expect((await parseOpenSCAD(text)).meshes).toHaveLength(1)
})
it('makes direct repeat-count parameters integer sliders',()=>{
 const c=compileModelGraphText('// @modelgraph-text/1\nparam count = 3 range 1..10\nbody = repeat(count, i => sphere(1).translate([i*3,0,0]))')
 expect(c.customizer[0]!.step).toBe(1)
 expect(c.document.parameters[0]!.integer).toBe(true)
})

it("accepts rect as the rectangle primitive",()=>{
 const source="// @modelgraph-text/1\nshow rect([12,8]).move([2,3,0]).extrude(4)"
 expect(compileModelGraphText(source).source).toBe(compileModelGraphText(source.replace("rect(","rectangle(")).source)
})
