import { expect, it } from 'vitest'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { setModelGraphParameters } from '../src/services/modelGraph'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { resolveInterval } from '../src/services/modelGraphRange'
import { magnitude } from '../src/services/modelGraphUnits'
const fail = (_code:string,_path:string,message:string):never => {throw new Error(message)}
const header='// @modelgraph-text/1\n'
const scene='\nbody = box([1mm,1mm,1mm])\nshow body\n'
const values=(start:number,end:number,inclusive:boolean,options:{count?:number;step?:number})=>resolveInterval(start,end,inclusive,options,fail,'/test').map(magnitude)
it('defines endpoints, count, step, singleton, empty and descending ranges',()=>{
 expect(values(0,360,false,{count:18})).toEqual(Array.from({length:18},(_,i)=>i*20))
 expect(values(0,360,true,{count:18}).at(-1)).toBe(360)
 expect(values(0,10,true,{step:3})).toEqual([0,3,6,9])
 expect(values(10,0,true,{step:-2})).toEqual([10,8,6,4,2,0])
 expect(values(10,0,true,{})).toEqual([])
 expect(values(0,0,false,{})).toEqual([])
 expect(values(0,10,true,{count:1})).toEqual([0])
 expect(values(0,10,true,{count:0})).toEqual([])
 expect(values(0,0.3,true,{step:0.1})).toHaveLength(4)
 expect(values(0,0.3,false,{step:0.1})).toHaveLength(3)
})
it('supports typed ranges, scalar comprehensions and operators',()=>{
 const c=compileModelGraphText(header+'positions = 0mm..1cm by 2mm\nsquares = [for i in 0..<6 where i % 2 == 0 let n = i ** 2 => n]\n'+scene+'assert length(positions). equalTo(6)\nassert at(squares,2). equalTo(16)\nassert at(positions,5). equalTo(10mm)')
 expect(c.constraint_report.every(r=>r.passed)).toBe(true)
})
it('supports exact-count parameter changes in canonical JSON',async()=>{
 const c=compileModelGraphText(header+'param count = 18 range 1..24\nplanet = box([1mm,1mm,1mm])\nangles = 0deg..<360deg count count\nplanets = [for angle in angles => planet.translate(x: 10mm).rotate(z: angle)]\nshow planets')
 expect(c.document.parameters[0]?.integer).toBe(true)
 const built=await parseOpenSCAD(c.source)
 expect(built.meshes).toHaveLength(18)
 const changed=setModelGraphParameters(c.document,c.document_sha256,[{id:'count',value:6}])
 expect((await parseOpenSCAD(changed.source)).meshes).toHaveLength(6)
 expect(()=>setModelGraphParameters(c.document,c.document_sha256,[{id:'count',value:1.5}])).toThrow()
})
it('supports nested loops, filtering and tuple zip/enumerate',async()=>{
 const c=compileModelGraphText(header+'angles = 0deg..<360deg count 4\nvalues = [for (i,a) in enumerate(angles) where i != 1 => a]\npairs = [for (x,y) in zip([1,2],[3,4]) => x+y]\n'+scene+'assert length(values). equalTo(3)\nassert at(pairs,1). equalTo(6)')
 expect(c.constraint_report.every(r=>r.passed)).toBe(true)
 const built=await parseOpenSCAD(header+'parts = [for x in 0..<3 for y in 0..<2 where x + y > 0 => box([1mm,1mm,1mm]).translate([x*3mm,y*3mm,0mm])]\nshow parts')
 expect(built.meshes).toHaveLength(5)
})
it('flattens nested numeric comprehensions in stable order',()=>{
 const c=compileModelGraphText(header+'values = [for x in 0..<3 for y in 0..<2 => x*10+y]'+scene+'assert length(values). equalTo(6)\nassert at(values,3). equalTo(11)')
 expect(c.constraint_report.every(r=>r.passed)).toBe(true)
})
it('preserves separate touching parts unless union is explicit',async()=>{
 const source=header+'parts = [for x in 0..<2 => box([1mm,1mm,1mm]).translate(x: x*1mm)]\n'
 expect((await parseOpenSCAD(source+'show parts')).meshes).toHaveLength(2)
 expect((await parseOpenSCAD(source+'show union(parts)')).meshes).toHaveLength(1)
})
it.each(['0mm..10mm','0mm..10deg count 3','0mm..10mm by 1deg','0..10 by 0','0..10 count 2.5','0..10 count 2mm','0..10 count 300','0..10000','0..10 by 2 count 3','0..10 count 3 by 2','0..0 count 1 by 1','0..<0 count 1'])('rejects invalid range %s',range=>{
 expect(()=>compileModelGraphText(header+'xs = '+range+scene+'assert length(xs). atLeast(0)')).toThrow()
})
it('rejects unequal zip lengths and implicit topology changes of collections',()=>{
 expect(()=>compileModelGraphText(header+'xs=zip([1,2],[1])'+scene+'assert length(xs). atLeast(0)')).toThrow('equal length')
 expect(()=>compileModelGraphText(header+'parts=[for i in 0..<3 => circle(1mm)]\nshow parts.extrude(2mm)')).toThrow('Transform each')
})

it('exports separate generated scene objects to 3MF',async()=>{
 const {flattenExportMeshes}=await import('../src/services/meshExportAdapter')
 const {exportMeshFormat}=await import('../src/services/meshExportFormats')
 const built=await parseOpenSCAD(header+'parts=[for i in 0..<3 => box([1mm,1mm,1mm]).translate(x: i*1mm)]\nshow parts')
 const xml=new TextDecoder().decode(exportMeshFormat(flattenExportMeshes(built.meshes),'3mf').data)
 expect(xml.match(/<object id=/g)).toHaveLength(3)
 expect(xml.match(/<item objectid=/g)).toHaveLength(3)
})
it('rejects oversized nested generation before geometry execution',()=>{
 expect(()=>compileModelGraphText(header+'parts=[for x in 0..<256 for y in 0..<256 => sphere(1mm)]\nshow parts')).toThrow(/budget|4096|allocation/i)
})
it('evaluates power with conventional precedence and right association',()=>{
 const c=compileModelGraphText(header+scene+'assert (-2 ** 2).equalTo(-4)\nassert (2 ** 3 ** 2).equalTo(512)')
 expect(c.constraint_report.every(r=>r.passed)).toBe(true)
})
