import { expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { compileModelGraph } from '../src/services/modelGraph'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { formatCode } from '../src/services/codeFormat'
const header = '// @modelgraph-text/1\n'
const compile = (s: string) => compileModelGraphText(header+s)
const same = (s: string, expected: string) => expect(compile(s).source).toBe(compile(expected).source)
const plate = 'struct Plate { width: length, depth: length, height: length }\np = Plate { width: 10mm, depth: 12mm, height: 4mm }\n'
const render = `trait Render {
  type Output
  fn render self: Self -> Self.Output
}
`
const implementation = `impl Render for Plate {
  type Output = Geometry
  fn render self: Self ret box(self.width,self.depth,self.height)
}
`
it('dispatches a required method and substitutes its associated result type', async () => {
 const source=render+plate+implementation+'fn draw[T: Render] value: T -> T.Output ret value.render()\nshow draw(p)'
 same(source,'show box(10mm,12mm,4mm)')
 expect((await parseOpenSCAD(header+source)).volume).toBeCloseTo(480)
})
it('uses structural default methods without requiring impl',()=>{
 same(`trait Sized {
  width: length
  fn solid self: Self -> Geometry ret box(self.width,12mm,4mm)
}
p = { width: 10mm, label: "p" }
show p.solid()`,'show box(10mm,12mm,4mm)')
})
it('overrides defaults and calls required methods from inherited defaults',()=>{
 same(`trait Sized {
  fn width self: Self -> length
  fn solid self: Self -> Geometry ret box(self.width(),12mm,4mm)
}
`+plate+`impl Sized for Plate {
  fn width self: Self ret self.width * 2
}
show p.solid()`,'show box(20mm,12mm,4mm)')
})
it('overrides a default body while retaining other defaults',()=>{
 same(`trait Sized {
  width: length
  fn size self: Self -> length ret self.width
  fn solid self: Self -> Geometry ret box(self.size(),12mm,4mm)
}
`+plate+`impl Sized for Plate {
  fn size self: Self -> length ret self.width + 5mm
}
show p.solid()`,'show box(15mm,12mm,4mm)')
})
it('keeps method bodies lexical, accepts named/default arguments and with Self',()=>{
 same(`extra = 5mm
trait Widen {
  width: length
  fn widen self: Self, by: length = extra -> Self ret self with { width: self.width + by }
}
`+plate+`fn caller value: Plate, extra: length ret value.widen()
show box(caller(p,99mm).width,p.widen(by:2mm).width,4mm)`,'show box(15mm,12mm,4mm)')
})
it('supports layout method bodies and assertions',()=>{
 same(`trait Solid {
  width: length
  fn solid self: Self -> Geometry
    assert(self.width > 0mm)
    height = 4mm
    ret box(self.width,12mm,height)
}
show {width:10mm}.solid()`,'show box(10mm,12mm,4mm)')
 expect(()=>compile(`trait Solid {
  width: length
  fn solid self: Self -> Geometry
    assert(self.width > 0mm)
    ret box(1,1,1)
}
show {width:-1mm}.solid()`)).toThrow()
})
it('supports concrete generic structure implementations',()=>{
 same(render+`struct Holder<T> { value: T }
impl Render for Holder<length> {
 type Output = Geometry
 fn render self: Self ret sphere(self.value)
}
show Holder<length> {value:3mm}.render()`,'show sphere(3mm)')
})
it('resolves associated input types and fields from generic bounds',()=>{
 same(`trait Value {
 type Item
 value: Self.Item
 fn replace self: Self, next: Self.Item -> Self ret self with {value:next}
}
struct Box { value: length }
impl Value for Box { type Item = length }
fn change[T: Value] value: T, next: T.Item -> T ret value.replace(next)
p = change(Box {value:2mm},5mm)
show sphere(p.value)`,'show sphere(5mm)')
})
it('retains parameter expressions in method results after graph edits',()=>{
 const doc=structuredClone(compile(render+`param width = 10mm range 1mm..30mm
struct Plate { width: length, depth: length, height: length }
p = Plate {width:width,depth:12mm,height:4mm}
`+implementation+'show p.render()').document)
 doc.parameters[0]!.value=20
 expect(compileModelGraph(doc).source).toBe(compile('show box(20mm,12mm,4mm)').source)
})
it.each([
 [render+plate+'impl Render for Plate { type Output = Geometry }\nshow box(1,1,1)', /missing method render/],
 [render+plate+'impl Render for Plate { fn render self: Self ret box(1,1,1) }\nshow box(1,1,1)', /associated types/],
 [render+plate+implementation+implementation+'show p.render()', /Duplicate impl/],
 [render+plate+'impl Render for Plate { type Output = Geometry\nfn render self: Self -> length ret 2mm }\nshow box(1,1,1)', /result type/],
 [render+plate+'impl Render for Plate { type Output = Geometry\nfn render self: Self, x: int ret box(1,1,1) }\nshow box(1,1,1)', /input count/],
 [render+plate+'impl Render for Plate { type Output = Geometry\nfn other self: Self ret box(1,1,1) }\nshow box(1,1,1)', /Unknown trait method/],
 [render+plate+'impl Render for Plate { type Output = Missing\nfn render self: Self ret box(1,1,1) }\nshow box(1,1,1)', /Unknown type Missing/],
 ['trait Bad { fn f value: int -> int }\nshow sphere(1)', /self: Self/],
 ['trait Bad { fn f self: Self ret 1 }\nshow sphere(1)', /explicit result type/],
 [render+plate+'fn draw[T: Render] value: T ret box(1,1,1)\nshow draw(p)', /requires impl Render/],
 [render+plate+'show p.render()', /No applicable trait method/],
 ['trait A { fn f self: Self -> Geometry ret box(1,1,1) }\ntrait B { fn f self: Self -> Geometry ret box(2,2,2) }\nshow {}.f()', /Ambiguous trait method/],
 [render+plate+implementation+'show p.render(self:p)', /receiver/],
 ['trait A { fn f self: Self -> Geometry ret self.f() }\nshow {}.f()', /expansion|nesting/],
 [render+plate+'fn draw[T: Render] value: T -> T.Missing ret value\nshow box(1,1,1)', /Unknown type T.Missing/],
] as const)('rejects invalid implementation: %s', (source,error)=>expect(()=>compile(source)).toThrow(error))
it('builds and formats every method guide example',async()=>{
 const guide=readFileSync(new URL('../docs/languages/modelgraph-text-trait-methods.md',import.meta.url),'utf8')
 const examples=[...guide.matchAll(/```text\n([\s\S]*?)```/g)].map(m=>m[1]!)
 expect(examples.length).toBeGreaterThan(2)
 for(const source of examples) {
  expect((await parseOpenSCAD(source)).meshes.length).toBeGreaterThan(0)
  expect(compileModelGraphText(formatCode(source)).source).toBe(compileModelGraphText(source).source)
 }
})
it('disambiguates through a generic bound or explicit trait call',()=>{
 same(`trait A { fn f self: Self -> Geometry ret box(1,2,3) }
trait B { fn f self: Self -> Geometry ret box(4,5,6) }
fn render[T: A] value: T ret value.f()
show [render({}), B.f({})]`,'show [box(1,2,3),box(4,5,6)]')
})
it('does not leak the caller method trait into an ordinary function',()=>{
 expect(()=>compile(`trait A { fn f self: Self -> Geometry ret box(1,2,3) }
trait B { fn f self: Self -> Geometry ret box(4,5,6) }
fn ambiguous p: int ret {}.f()
trait Outer { fn build self: Self -> Geometry ret ambiguous(1) }
show {}.build()`)).toThrow(/Ambiguous/)
})
it('resolves associated aliases regardless of declaration order',()=>{
 same(`trait Value {
 type Items
 type Item
 fn values self: Self -> Self.Items
}
struct Thing { radius: length }
impl Value for Thing {
 type Items = Vec<Self.Item>
 type Item = length
 fn values self: Self ret [self.radius,2mm]
}
p = Thing {radius:3mm}
show sphere(p.values()[0])`,'show sphere(3mm)')
})
it('rejects cycles in associated aliases',()=>{
 expect(()=>compile(`trait A {type X\ntype Y}
struct P {x:int}
impl A for P {type X = Self.Y\ntype Y = Self.X}
show box(1,1,1)`)).toThrow(/Cyclic associated type/)
})
it('enforces the declared result even when impl infers it',()=>{
 expect(()=>compile(render+plate+`impl Render for Plate {
 type Output = length
 fn render self: Self ret "wrong"
}
show sphere(p.render())`)).toThrow()
})
it('inherits argument defaults in overrides',()=>{
 same(`trait Shape { fn solid self: Self, width: length = 5mm -> Geometry ret box(width,2mm,3mm) }
struct P {unused:int}
impl Shape for P { fn solid self: Self, width: length ret box(width*2,2mm,3mm) }
show P {unused:1}.solid()`,'show box(10mm,2mm,3mm)')
})
it('rejects conflicting associated names in multiple generic bounds',()=>{
 expect(()=>compile(`trait A {type Item}
trait B {type Item}
struct P {x:int}
impl A for P {type Item=length}
impl B for P {type Item=str}
fn use[T: A + B] value: T -> T.Item ret 2mm
show sphere(use(P {x:1}))`)).toThrow(/Ambiguous associated type/)
})
it('keeps distinct generic receiver bounds when concrete types coincide',()=>{
 same(`trait A { fn solid self: Self -> Geometry ret box(1,2,3) }
trait B { fn solid self: Self -> Geometry ret box(4,5,6) }
fn pair[T: A, U: B] left: T, right: U
 copy = left with {}
 ret [copy.solid(),right.solid()]
show pair({},{})`,'show [box(1,2,3),box(4,5,6)]')
})
it('builds an own-nurbs geometry returned by a trait method',async()=>{
 const source=`trait Solid { fn build self: Self -> Geometry ret brep_box([0,0,0],[2mm,3mm,4mm]).brep_tessellate(4) }
show {}.build()`
 expect(compile(source).execution_target).toBe('own-nurbs')
 expect((await parseOpenSCAD(header+source)).volume).toBeCloseTo(24)
})
it('rejects type arguments on associated aliases',()=>{
 expect(()=>compile(`trait A {type X}
struct P {x:int}
impl A for P {type X=Self<int>}
show sphere(1)`)).toThrow(/no type arguments/)
})
