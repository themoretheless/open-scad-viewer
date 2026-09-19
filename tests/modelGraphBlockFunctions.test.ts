import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { compileModelGraph } from '../src/services/modelGraph'
import { parseOpenSCAD } from '../src/services/openscadParser'
const header='// @modelgraph-text/1\n'
const compile=(text:string)=>compileModelGraphText(header+text)
const source=readFileSync('examples/modelgraph-text/generic-functions.mg','utf8')

it('builds the agreed generic column syntax and preserves parameter dependencies',async()=>{
 const c=compileModelGraphText(source)
 expect(c.source).toContain('cube([10,20,12]')
 const doc=structuredClone(c.document);doc.parameters[0]!.value=25
 expect(compileModelGraph(doc).source).toContain('cube([10,20,25]')
 const built=await parseOpenSCAD(source)
 expect(built.volume).toBeCloseTo(2400)
})
it('accepts explicit generics, named arguments, records, shorthand and local bindings',()=>{
 const text=`struct Pair<T> { a: T, b: T }
 make = fn<T>
  first: T,
  second: T,
 -> pair: Pair<T>, count: int,
 {
  n = 2
  ret { count: n, pair: Pair<T> { a: first, b: second } }
 }
 result = make<f32>(second: 4, first: 2)
 show box([result.pair.a, result.pair.b, result.count])`
 expect(compile(text).source).toContain('cube([2,4,2]')
})
it('supports geometric results, lexical capture and calls from generators',()=>{
 const text=`param width = 2 range 1..4
 make = fn
  i: int
 -> body: Geometry, count: int
 {
  size = width + i
  ret { body: box([size,1,1]), count: 1 }
 }
 show [for i in 0..<3 => make(i).body.translate([i*5,0,0])]`
 const c=compile(text)
 expect(c.source).toContain('cube([2,1,1]')
 expect(c.source).toContain('cube([4,1,1]')
})
it('supports local destructuring and nested calls',()=>{
 const text=`inner = fn x: int -> n: int { ret {n:x+1} }
 outer = fn x: int -> n: int {
 { n } = inner(x)
 ret { n }
 }
 { n } = outer(2)
 show sphere(n)`
 expect(compile(text).source).toContain('sphere(r=3')
})
it('does not reinterpret less-than comparisons as generic arguments',()=>{
 expect(compile('a = 1\nshow a < 2 ? sphere(1) : sphere(2)').source).toContain('sphere(r=1')
})
it.each([
 ['wrong argument type','f = fn x: int -> n: int { ret {n:x} }\nr = f(1.5)\nshow sphere(1)'],
 ['missing result','f = fn -> a:int, b:int { ret {a:1} }\nr=f()\nshow sphere(1)'],
 ['extra result','f = fn -> a:int { ret {a:1,b:2} }\nr=f()\nshow sphere(1)'],
 ['unknown type','f = fn x:Missing -> a:int { ret {a:1} }\nshow sphere(1)'],
 ['duplicate parameter','f = fn x:int, x:int -> a:int {ret {a:1}}\nshow sphere(1)'],
 ['duplicate result field','f = fn -> a:int {ret {a:1,a:2}}\nshow sphere(1)'],
 ['missing return','f = fn -> a:int {}\nshow sphere(1)'],
 ['unknown member','r = {a:1}\nshow sphere(r.b)'],
 ['unknown destructured member','{b} = {a:1}\nshow sphere(1)'],
 ['duplicate binding','a=1\n{a}={a:2}\nshow sphere(a)'],
 ['conflicting generic inference','f=fn<T> x:T, y:T -> a:T {ret {a:x}}\nr=f(1,"s")\nshow sphere(1)'],
 ['unknown generic parameter','f=fn<T> x:T -> a:U {ret {a:x}}\nshow sphere(1)'],
 ['missing record field','struct P {x:f32,y:f32}\np=P{x:1}\nshow sphere(1)'],
 ['nominal type mismatch','struct A{x:int}\nstruct B{x:int}\nf=fn p:A -> n:int {ret {n:p.x}}\nr=f(B{x:1})\nshow sphere(1)'],
 ['recursive struct','struct A {child:A}\nshow sphere(1)'],
 ['recursion','f=fn x:int -> n:int {ret {n:f(x).n}}\nr=f(1)\nshow sphere(r.n)'],
 ['unexpected eof','f=fn x:int -> n:int {ret {n:x}'],
])('rejects %s',(_name,text)=>expect(()=>compile(text)).toThrow())
it('validates int after canonical parameter updates',()=>{
 const c=compile('param p=2 range 1..4\nf=fn x:int -> n:int {ret {n:x}}\nr=f(p)\nshow sphere(r.n)')
 const doc=structuredClone(c.document);doc.parameters[0]!.value=2.5
 expect(()=>compileModelGraph(doc)).toThrow(/int/)
})
it('supports numeric typed fields on the own backend',()=>{
 const c=compile('f=fn x:f32 -> size:f32 {ret {size:x}}\nr=f(2)\nshow brep_box([0,0,0],[r.size,3,4]).brep_tessellate(1)')
 expect(c.execution_target).toBe('own-nurbs')
})
it('checks every numeric argument and named result when a function result is used',()=>{
 expect(()=>compile('f=fn x:int -> body:Geometry {ret {body:sphere(1)}}\nr=f(1/2)\nshow r.body')).toThrow(/int/)
 expect(()=>compile('f=fn -> body:Geometry, count:int {ret {body:sphere(1),count:1/2}}\nr=f()\nshow r.body')).toThrow(/int/)
 expect(()=>compile('param p=2 range 1..4\nf=fn x:int -> body:Geometry {ret {body:sphere(1)}}\nr=f(p)\nshow r.body').document).not.toThrow()
})

it('returns a single generic value from short bodies with inferred and explicit types',()=>{
 const c=compile(`identity = fn<T> value: T -> T => value
 show box([identity(2), identity<f64>(3), identity(4)])`)
 expect(c.source).toContain('cube([2,3,4]')
})
it('returns geometry directly from a block and chains methods',()=>{
 const c=compile(`make = fn size: f64 -> Geometry {
 base = rectangle([size,size])
 ret base.extrude(3)
 }
 show make(2).translate([1,0,0])`)
 expect(c.source).toContain('linear_extrude(height=3')
})
it('supports short named results and direct nominal record results',()=>{
 const c=compile(`struct Pair<T> { a:T, b:T }
 pair = fn<T> x:T -> Pair<T> => Pair<T> {a:x,b:x}
 named = fn x:int -> first:int, second:int => {first:x,second:x+1}
 {first,second} = named(2)
 show box([pair(3).a,first,second])`)
 expect(c.source).toContain('cube([3,2,3]')
})
it.each([
 'f = fn -> Geometry => 1\nshow f()',
 'f = fn -> int { ret 1.5 }\nshow sphere(f())',
 'f = fn -> Missing => 1\nshow sphere(1)',
 'f = fn -> int =>\n',
 'f = fn -> int { ret {value:1} }\nshow sphere(f())',
])('rejects invalid direct returns: %s',text=>{
 expect(()=>compile(text)).toThrow()
})

it.each([
 'ret x1: b, x2: 5,\n        x3: 55, x4: 6546',
 'ret\n        x1: b, x2: 5,\n        x3: 55, x4: 6546',
])('supports layout functions and mixed return lines: %s',ret=>{
 const c=compile(`fn get_first[T] a: int, b: T -> x1: T, x2: int, x3: int, x4: int
    ${ret}
r = get_first(1, 3)
show box([r.x1,r.x2,r.x3])`)
 expect(c.source).toContain('cube([3,5,55]')
})
it('supports column signatures, blank lines, local bindings and direct returns',()=>{
 const c=compile(`fn get_first[T]
    a: int,
    b: T
->
    x1: T,
    x2: int

    sum = a + 2
    ret
        x1: b,
        x2: sum

fn solid size: int -> Geometry
    ret box([size,2,3])
r = get_first(1,4)
show solid(r.x2)`)
 expect(c.source).toContain('cube([3,2,3]')
})
it('preserves parameter spans and canonical updates through layout functions',()=>{
 const text=header+`param size = 3 range 1..8
fn identity[T] value: T -> T
    ret value
show box([identity(size),2,3])`
 const c=compileModelGraphText(text),control=c.customizer[0]!
 expect(text.slice(control.valueStart,control.valueEnd)).toBe('3')
 const doc=structuredClone(c.document);doc.parameters[0]!.value=7
 expect(compileModelGraph(doc).source).toContain('cube([7,2,3]')
})
it.each([
 'fn f -> a:int,\n    ret a:1',
 'fn f -> a:int\nret a:1',
 'fn f -> a:int, b:int\n    ret a:1\n        b:2',
 'fn f -> a:int\n    ret a:1,\nshow sphere(1)',
 'fn f -> a:int\n    ret\n    a:1',
 'fn f -> a:int\n    ret a:1\n    x:2',
 'fn f -> a:int\n \tret a:1',
])('rejects malformed layout: %s',text=>{
 expect(()=>compile(text+'\nshow sphere(1)')).toThrow()
})
it('builds the indented functions example as a solid',async()=>{
 const text=readFileSync('examples/modelgraph-text/indented-functions.mg','utf8')
 const built=await parseOpenSCAD(text)
 expect(built.volume).toBeCloseTo(2400)
})
it('accepts colon parameters with units and preserves editable spans',()=>{
 const text=header+'param size: -2mm range -5mm..5mm\nshow sphere(size + 4mm)'
 const c=compileModelGraphText(text),p=c.customizer[0]!
 expect(text.slice(p.valueStart,p.valueEnd)).toBe('-2')
 expect(c.source).toContain('sphere(r=2')
})
it('supports no-result functions ending at dedent and calls inside functions',()=>{
 const c=compile(`fn helper width: f64, depth: f64
    b = width + depth

fn outer size: f64
    helper(size, size)
    ret

outer(2)
show sphere(1)`)
 expect(c.source).toContain('sphere(r=1')
})
it('allows an unused no-result declaration at end of source',()=>{
 expect(compile('show sphere(1)\nfn helper x: int\n    b = x + 1').source).toContain('sphere(r=1')
})
it.each([
 'fn f x: int\n    b = x\nr = f(1)\nshow sphere(1)',
 'fn f x: int\n    b = x\nf(1.5)\nshow sphere(1)',
])('rejects invalid no-result use: %s',text=>expect(()=>compile(text)).toThrow())
