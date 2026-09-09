import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { formatCode } from '../src/services/codeFormat'
import { compileModelGraph } from '../src/services/modelGraph'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { parseOpenSCAD } from '../src/services/openscadParser'

const header = '// @modelgraph-text/1\n'
const compile = (source: string) => compileModelGraphText(header + source)
const build = (source: string) => parseOpenSCAD(header + source)
const same = (source: string, expected: string) => expect(compile(source).source).toBe(compile(expected).source)

describe('concise function declarations and calls', () => {
  it('builds every complete guide example before and after formatting', async () => {
    const guide = readFileSync(new URL('../docs/languages/modelgraph-text-concise.md', import.meta.url), 'utf8')
    const examples = [...guide.matchAll(/```text\n([\s\S]*?)```/g)].map(match => match[1]!)
    expect(examples).toHaveLength(7)
    for (const source of examples) {
      const scene = await parseOpenSCAD(source)
      expect(scene.meshes.length).toBeGreaterThan(0)
      expect(compileModelGraphText(formatCode(source)).source).toBe(compileModelGraphText(source).source)
    }
  })

  it('keeps default and inferred functions compatible with the own-nurbs route', async () => {
    const source = 'fn body size: length = 2mm => brep_box([0,0,0],[size,size,size]).brep_tessellate(4)\nshow body()'
    expect(compile(source).execution_target).toBe('own-nurbs')
    expect((await build(source)).volume).toBeCloseTo(8)
  })
  it('uses defaults, named overrides and earlier arguments in declaration scope', () => {
    const definition = `base = 3mm
fn plate width: length, depth: length = width / 2, height: length = base -> Geometry
  ret box([width,depth,height])
fn caller base: length -> Geometry
  ret plate(20mm)
`
    same(definition + 'show caller(99mm)', 'show box([20mm,10mm,3mm])')
    same(definition + 'show plate(height: 5mm, width: 20mm)', 'show box([20mm,10mm,5mm])')
    same(definition + 'show plate(20mm,8mm,6mm)', 'show box([20mm,8mm,6mm])')
  })

  it('keeps parameter-dependent defaults after canonical parameter edits', () => {
    const source = `param height = 3mm range 1mm..8mm
fn plate width: length = 10mm, thickness: length = height => box([width,8mm,thickness])
show plate()`
    const compiled = compile(source)
    const document = structuredClone(compiled.document)
    document.parameters[0]!.value = 6
    expect(compileModelGraph(document).source).toBe(compile('show box([10mm,8mm,6mm])').source)
  })

  it('does not evaluate an overridden default', () => {
    const definition = 'fn value x: f64 = 1 / 0 => x\n'
    same(definition + 'show sphere(value(2))', 'show sphere(2)')
    expect(() => compile(definition + 'show sphere(value())')).toThrow()
  })

  it('supports generic defaults and inferred result types', () => {
    const definition = 'fn same[T] x: T, y: T = x => y\n'
    same(definition + 'show sphere(same(2mm))', 'show sphere(2mm)')
    same(definition + 'show sphere(same<length>(3mm))', 'show sphere(3mm)')
    same('fn square value: int => value * value\nshow sphere(square(3))', 'show sphere(9)')
  })

  it.each([
    'make = fn width: length = 20mm => box([width,12mm,4mm])',
    'make = fn width: length = 20mm { assert width > 0mm; ret box([width,12mm,4mm]) }',
    'fn make width: length = 20mm -> Geometry => box([width,12mm,4mm])',
  ])('supports compact and brace definitions: %s', definition => {
    same(definition + '\nshow make()', 'show box([20mm,12mm,4mm])')
  })

  it.each([
    'plate()', 'plate(1mm,2mm,3mm)', 'plate(width:1mm, width:2mm)',
    'plate(1mm, depth:2mm)', 'plate(width:1mm, unknown:2mm)', 'plate(1deg)',
  ])('rejects invalid binding or units: %s', call => {
    expect(() => compile('fn plate width: length, depth: length = 2mm => box([width,depth,1mm])\nshow ' + call)).toThrow()
  })

  it('retains no-result function semantics and assertion failures', () => {
    const source = `fn requirePositive size: length
  assert(size > 0mm).message("Positive default")
fn plate size: length = 2mm
  requirePositive(size)
plate()`
    same(source + '\nshow sphere(1mm)', 'show sphere(1mm)')
    expect(() => compile(source.replace('plate()', 'plate(-1mm)') + '\nshow sphere(1mm)')).toThrow('Positive default')
  })
})

describe('named callbacks', () => {
  it('passes items to functions and keeps optional arguments at their defaults', async () => {
    const source = `fn plate height: length, width: length = 5mm => box([width,2mm,height])
show [3mm,4mm].select(plate)`
    const scene = await build(source)
    expect(scene.meshes).toHaveLength(2)
    expect(scene.volume).toBeCloseTo(70)
  })

  it('passes the optional callback index only when explicitly required', () => {
    same(`fn plate height: length, index: int, spacing: length = 3mm => box([1mm,1mm,height]).move(x: index*spacing)
show [2mm,4mm].select(plate)`, 'show [box([1mm,1mm,2mm]).move(x:0mm),box([1mm,1mm,4mm]).move(x:3mm)]')
  })

  it('uses named callbacks in repeat, predicates and reducers', () => {
    same(`fn part index: int => sphere(1mm).move(x:index*3mm)
show repeat(3,part)`, 'show repeat(3,i => sphere(1mm).move(x:i*3mm))')
    same(`fn positive value: int => value > 0
fn add left: int, right: int => left + right
show sphere([-1,2,3].where(positive).aggregate(0,add))`, 'show sphere(5)')
  })

  it('preserves assertion scopes and rejects wrong callback arity', () => {
    expect(() => compile('fn part index: int => box([index,1,1])\nshow repeat(2,part)')).toThrow()
    expect(() => compile('fn part x: int, y: int, z: int => sphere(1)\nshow [1].select(part)')).toThrow('Callback')
    expect(() => compile('fn part x: int, y: int => sphere(1)\nshow repeat(2,part)')).toThrow('one index')
    expect(() => compile('fn part x: int -> Geometry\n  assert(x > 0).message("Callback assertion")\n  ret sphere(x)\nshow [1,0].select(part)')).toThrow('Callback assertion')
  })
})

describe('list indexing and destructuring', () => {
  it('indexes nested lists and records, with normal expression precedence', () => {
    same('items = [{sizes:[2mm,3mm]},{sizes:[4mm,5mm]}]\nshow box([items[1].sizes[0],items[0].sizes[1],1mm])', 'show box([4mm,3mm,1mm])')
    same('sizes=[2,4,6]\nshow sphere(sizes[1+1]*2)', 'show sphere(12)')
  })

  it('recomputes dynamic indices and rejects invalid indices', () => {
    const compiled = compile('param index = 0 range 0..1\nshow sphere([2mm,4mm][index])')
    const document = structuredClone(compiled.document)
    document.parameters[0]!.value = 1
    expect(compileModelGraph(document).source).toBe(compile('show sphere(4mm)').source)
    for (const index of ['-1','2','0.5','1mm']) expect(() => compile(`show sphere([2mm,4mm][${index}])`)).toThrow()
  })

  it('selects a geometry item without displaying the other items', async () => {
    const scene = await build('parts=[box([1,1,1]),box([2,2,2])]\nshow parts[1].move(x:3mm)')
    expect(scene.meshes).toHaveLength(1)
    expect(scene.volume).toBeCloseTo(8)
  })

  it('destructures literal and dynamic lists at top level and inside functions', () => {
    same('[width,depth,height]=[20mm,12mm,4mm]\nshow box([width,depth,height])', 'show box([20mm,12mm,4mm])')
    same(`fn part sizes: Vec<length> -> Geometry
  [width,depth,height] = sizes
  ret box([width,depth,height])
show part([20mm,12mm,4mm])`, 'show box([20mm,12mm,4mm])')
    expect(() => compile('[x,y]=[1,2,3]\nshow sphere(x)')).toThrow('exactly 2')
    expect(() => compile('[x,x]=[1,2]\nshow sphere(x)')).toThrow('duplicate')
  })

  it('enforces dynamic list length even when the bound values are unused', () => {
    const source = 'fn part sizes: Vec<int> -> Geometry\n  [x,y] = sizes\n  ret sphere(1)\nshow part([1,2,3])'
    expect(() => compile(source)).toThrow('exactly 2')
    expect(() => compile('items=(0..<3).toArray()\n[x,y]=items\nshow sphere(1)')).toThrow('exactly 2')
  })

  it('destructures in foreach and match blocks', () => {
    same(`parts = foreach sizes in [[2mm,3mm],[4mm,5mm]]
  [width,height] = sizes
  yield box([width,1mm,height])
show parts`, 'show [box([2mm,1mm,3mm]),box([4mm,1mm,5mm])]')
    same(`show match 1
  1 =>
    [width,height] = [2mm,3mm]
    ret box([width,1mm,height])
  _ => sphere(1mm)`, 'show box([2mm,1mm,3mm])')
  })
})

describe('collection transformations', () => {
  it('allows transformations of empty literal and generated collections', () => {
    expect(compile('show [].move(z:1mm)').source).not.toContain('cube')
    expect(compile('show [for i in 0..<0 => box([1,1,1])].move(z:1mm)').source).not.toContain('cube')
  })
  it.each([
    '[box([1mm,1mm,1mm]),box([1mm,1mm,1mm]).move(x:1mm)]',
    '[for i in 0..<2 => box([1mm,1mm,1mm]).move(x:i*1mm)]',
  ])('preserves separate touching objects in %s', async parts => {
    const source = `parts=${parts}\nshow parts.move(z:10mm).rotate(z:90deg).scale(x:2).mirror([0,1,0])`
    const scene = await build(source)
    expect(scene.meshes).toHaveLength(2)
    expect(scene.volume).toBeCloseTo(4)
    expect(compile(source).source).not.toContain('union')
  })

  it('keeps checks on the original collection independent of later transforms', async () => {
    const source = `parts=[box([1mm,1mm,2mm]),box([1mm,1mm,2mm]).move(x:3mm)]
assert measure(parts).height.approximately(2mm)
show parts.scale(z:2)`
    expect((await build(source)).meshes).toHaveLength(2)
    await expect(build(source.replace('approximately(2mm)', 'approximately(4mm)'))).rejects.toThrow('Geometry assertions')
  })
})
