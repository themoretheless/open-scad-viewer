import { expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { compileModelGraph } from '../src/services/modelGraph'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { formatCode } from '../src/services/codeFormat'

const header = '// @modelgraph-text/1\n'
const compile = (s: string) => compileModelGraphText(header + s)
const same = (s: string, expected: string) => expect(compile(s).source).toBe(compile(expected).source)
const sized = 'trait Sized { width: length, depth: length, height: length }\n'
const base = 'base = { width: 10mm, depth: 12mm, height: 4mm, label: "plate" }\n'
const widen = 'fn widen[T: Sized] value: T, extra: length = 5mm -> T ret value with { width: value.width + extra }\n'

it.each([
 'fn plate width: length ret box(width,12mm,4mm)',
 'fn plate width: length -> Geometry ret box(width,12mm,4mm)',
 'fn plate width: length ret\n  box(width,12mm,4mm)',
 'fn plate width: length\n  depth = 12mm\n  ret box(width,depth,4mm)',
])('supports parenthesis-free ret functions: %s', declaration => {
 same(declaration+'\nshow plate(10mm)', 'show box(10mm,12mm,4mm)')
})
it('copies anonymous records, retains extra fields and leaves the original unchanged', () => {
 same(sized+base+widen+`wide = widen(base)
assert(base.width == 10mm)
assert(wide.label == "plate")
show box(wide.width,wide.depth,wide.height)`, 'show box(15mm,12mm,4mm)')
})
it('retains nominal structure types through a generic with and explicit calls', () => {
 same(sized+`struct Plate { width: length, depth: length, height: length, code: str }
fn render part: Plate ret box(part.width,part.depth,part.height)
`+widen+`p = Plate { width: 10mm, depth: 12mm, height: 4mm, code: "p" }
show render(widen<Plate>(p))`, 'show box(15mm,12mm,4mm)')
})
it('supports multiple bounds', async () => {
 const source = sized+`trait Positioned { x: length }
fn render[T: Sized + Positioned] p: T ret box(p.width,p.depth,p.height).move(x:p.x)
p = { width: 2mm, depth: 3mm, height: 4mm, x: 0mm }
show [render(p), render(p with { x: 10mm })]`
 expect((await parseOpenSCAD(header+source)).meshes).toHaveLength(2)
})
it('supports nested explicit updates and chained copies', () => {
 same(`part = { size: { width: 2mm, depth: 3mm }, height: 4mm }
updated = part with { size: part.size with { width: 5mm } } with { height: 6mm }
show box(updated.size.width, updated.size.depth, updated.height)`, 'show box(5mm,3mm,6mm)')
})
it('retains parameter expressions through canonical edits', () => {
 const doc = structuredClone(compile(sized+`param width = 10mm range 1mm..30mm
p = { width: width, depth: 12mm, height: 4mm }
`+widen+`q = widen(p)
show box(q.width,q.depth,q.height)`).document)
 doc.parameters[0]!.value = 20
 expect(compileModelGraph(doc).source).toBe(compile('show box(25mm,12mm,4mm)').source)
})
it('preserves generic nominal fields including empty typed lists', () => {
 same(`struct Part<T> { value: T, tags: Vec<str> }
fn copy[T] p: Part<T> -> Part<T> ret p with { tags: ["wide"] }
p = Part<length> { value: 4mm, tags: [] }
q = copy(p)
assert(q.tags[0] == "wide")
show sphere(q.value)`, 'show sphere(4mm)')
})
it('checks constraints in nested generic vectors', () => {
 expect(()=>compile(sized+'fn use[T: Sized] values: Vec<T> ret box(1,1,1)\nshow use([{width:2mm}])')).toThrow(/depth|height/)
})
it.each([
 [sized+'fn unused[T: Missing] p: T ret p\nshow box(1,1,1)', /Unknown trait Missing/],
 [sized+'fn use[T: Sized] p: T ret box(1,1,1)\nshow use(2)', /record/],
 [sized+'fn use[T: Sized] p: T ret box(1,1,1)\nshow use({width:2mm})', /depth|height/],
 [sized+'fn use[T: Sized] p: T ret box(1,1,1)\nshow use({width:2deg,depth:3mm,height:4mm})', /width.*Sized|angle|length/],
 [sized+'fn use[T: Sized] p: Vec<T> ret box(1,1,1)\nshow use<int>([])', /record/],
 ['trait A { x: length }\ntrait B { x: str }\nfn use[T: A + B] p: T ret box(1,1,1)\nshow use({x:2mm})', /x.*B|str/],
 [base+'q = base with { missing: 1 }\nshow box(1,1,1)', /unknown field missing/],
 [base+'q = base with { width: "wrong" }\nshow box(1,1,1)', /scalar|length/],
 [base+'q = base with { label: 1 }\nshow box(q.width,q.depth,q.height)', /str/],
 [base+'q = base with { width: 2deg }\nshow box(q.width,q.depth,q.height)', /length|angle/],
 [base+'q = base with { width: 1mm, width: 2mm }\nshow box(1,1,1)', /Duplicate/],
 ['q = 1 with { x: 2 }\nshow box(1,1,1)', /record/],
 ['trait A { x: length, x: str }\nshow box(1,1,1)', /Duplicate/],
 ['trait A { x: Missing }\nshow box(1,1,1)', /Unknown type/],
 ['trait A { x: length }\nstruct A { x: length }\nshow box(1,1,1)', /Duplicate/],
 ['trait A { x: length }\nfn f[T: A + A] p: T ret p\nshow box(1,1,1)', /Duplicate/],
] as const)('rejects invalid traits or updates: %s', (source,error) => { expect(()=>compile(source)).toThrow(error) })
it('keeps assertions on overwritten values', () => {
 expect(()=>compile(`fn positive x: length
  assert(x > 0mm)
  ret x
p = { width: positive(-1mm) }
q = p with { width: 2mm }
show sphere(q.width)`)).toThrow()
})
it('builds and formats every guide example', async () => {
 const guide = readFileSync(new URL('../docs/languages/modelgraph-text-traits.md', import.meta.url),'utf8')
 const examples = [...guide.matchAll(/```text\n([\s\S]*?)```/g)].map(m=>m[1]!)
 expect(examples.length).toBeGreaterThan(2)
 for(const source of examples) {
   expect((await parseOpenSCAD(source)).meshes.length).toBeGreaterThan(0)
   expect(compileModelGraphText(formatCode(source)).source).toBe(compileModelGraphText(source).source)
 }
})
it('infers physical fields from arithmetic and retains their units', () => {
 same(sized+`p = { width: 2 * (3mm + 2mm), depth: 24mm / 2, height: -(-4mm) }
`+widen+`q = widen(p)
show box(q.width,q.depth,q.height)`, 'show box(15mm,12mm,4mm)')
})
it('validates an updated field even when that field is not used by geometry', () => {
 expect(()=>compile(base+'q = base with { width: 2deg }\nshow box(1,1,1)')).toThrow()
})
it('evaluates update expressions in the original lexical scope', () => {
 same('p = { x: 2mm, y: 3mm }\nq = p with { x: p.y, y: p.x }\nshow box(q.x,q.y,4mm)', 'show box(3mm,2mm,4mm)')
})
it('preserves no-result functions with a bare ret', () => {
 same('fn check x: int\n  ret\ncheck(1)\nshow sphere(1)', 'show sphere(1)')
})
it('rejects a return outside the function indentation', () => {
 expect(()=>compile('fn f x: int\nret x\nshow sphere(1)')).toThrow(/indent/)
})
