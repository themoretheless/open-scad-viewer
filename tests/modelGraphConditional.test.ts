import { expect, it } from 'vitest'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { compileModelGraph } from '../src/services/modelGraph'
import { parseOpenSCAD } from '../src/services/openscadParser'
const compile=(source:string)=>compileModelGraphText('// @modelgraph-text/1\n'+source)

it('selects scalar branches lazily and keeps parameter-dependent canonical expressions',()=>{
 const c=compile('param small = 1 range 0..1\nr = small ? 2mm : 1mm / 0\nshow sphere(r)')
 expect(c.source).toContain('sphere(r=2')
 const doc=structuredClone(c.document)
 doc.parameters[0]!.value=0
 expect(()=>compileModelGraph(doc)).toThrow()
 expect(compile('show sphere(0 ? 1mm / 0 : 3mm)').source).toContain('sphere(r=3')
})
it('supports right association, precedence, multiline branches and named arguments',()=>{
 expect(compile('show sphere(radius: 1 + 2 == 3 ? 0 ? 7 : 4 : 9)').source).toContain('sphere(r=4')
 expect(compile('r = 0 ?\n 2 :\n 3\nshow sphere(r)').source).toContain('sphere(r=3')
 expect(compile('show box(1 ? [1,2,3] : [4,5,6])').source).toContain('cube([1,2,3]')
})
it('selects geometry lazily, preserving the choice after parameter changes',async()=>{
 const c=compile('param round = 1 range 0..1\nshow round ? sphere(2) : box([2,3,4])')
 expect(c.source).toContain('sphere(')
 expect(c.source).not.toContain('cube(')
 const doc=structuredClone(c.document);doc.parameters[0]!.value=0
 expect(compileModelGraph(doc).source).toContain('cube(')
 const result=await parseOpenSCAD('// @modelgraph-text/1\nshow 0 ? sphere(-1) : box([2,3,4])')
 expect(result.volume).toBeCloseTo(24)
})
it('evaluates choices separately for generator bindings and functions',()=>{
 const c=compile('choose = x => x == 0 ? box([1,1,1]) : sphere(2)\nshow [for i in 0..<2 => choose(i)]')
 expect(c.source).toContain('cube(');expect(c.source).toContain('sphere(')
 expect(compile('ns = [for i in 0..<3 => i == 0 ? 2 : 4]\nshow sphere(at(ns,0))').source).toContain('sphere(r=2')
})
it('rejects malformed expressions, dimensional conditions and incompatible branch categories',()=>{
 for(const source of ['show 1 ? sphere(2)', 'show ? sphere(2) : sphere(3)', 'show 1 ? sphere(2) :', 'show 1mm ? sphere(2) : sphere(3)', 'show 1 ? sphere(2) : 4'])expect(()=>compile(source)).toThrow()
})
it('resolves lazy scalar and geometry choices on the own geometry backend',()=>{
 const c=compile('param choose = 1 range 0..1\nshow choose ? brep_box([0,0,0],[choose ? 2 : 1/0,3,4]).brep_tessellate(1) : brep_box([0,0,0],[-1,-1,-1]).brep_tessellate(1)')
 expect(c.execution_target).toBe('own-nurbs')
 expect(c.document.nodes.some(n=>n.op==='if')).toBe(false)
})

it('continues before question marks without consuming the next statement',()=>{
 const compact='param part = 0 range 0..2\nbody = box([2,3,4])\nhook = sphere(1)\nshow part == 2 ? hook : part == 1 ? body : [body,hook]'
 const multiline=compact.replace('show part == 2 ? hook : part == 1 ? body : [body,hook]', 'show part == 2\n  ? hook\n  : part == 1\n    ? body\n    : [body,hook]')
 for(const part of [0,1,2]) {
  expect(compile(multiline.replace('part = 0',`part = ${part}`)).source).toBe(compile(compact.replace('part = 0',`part = ${part}`)).source)
 }
 expect(compile('r = 1 + 2 == 3\n // continuation\n\n ? 4\n : 8\nx = 2\nshow sphere(r+x)').source).toContain('sphere(r=6')
 expect(compile('fn choose x: int -> Geometry\n  ret x > 0\n    ? sphere(2)\n    : box([1,1,1])\nshow choose(1)').source).toContain('sphere(r=2')
 expect(()=>compile('r = 1;\n? 2 : 3\nshow sphere(r)')).toThrow()
})
