import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { compileModelGraph } from '../src/services/modelGraph'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { parseOpenSCAD } from '../src/services/openscadParser'

const header = '// @modelgraph-text/1\n'
const compile = (source: string) => compileModelGraphText(header + source)
const build = (source: string) => parseOpenSCAD(header + source)

describe('ModelGraph scoped scalar assertions', () => {
  it('retains checks when every geometry match arm returns an empty list', () => {
    const source = `show match 1
  1 =>
    assert(1 == 1).message("Selected empty arm")
    ret []
  _ =>
    assert 2 == 2
    ret []`
    expect(compile(source).source).not.toContain('cube')
    expect(() => compile(source.replace('assert(1 == 1)', 'assert(1 == 0)'))).toThrow('Selected empty arm')
  })
  it.each(['assert', 'validate'])('checks plain predicates and messages with %s', keyword => {
    const source = `param size = 2 range 0..4
${keyword}(size > 0).message("Positive size")
${keyword} size < 4
show box([size,3,4])`
    expect(compile(source).source).toBe(compile('show box([2,3,4])').source)
    expect(() => compile(source.replace('size = 2', 'size = 0'))).toThrow('Positive size')
    expect(() => compile(source.replace('size = 2', 'size = 4'))).toThrow()
  })

  it('checks brace function arguments and locals when called', () => {
    const definition = `double = fn value: int -> int {
  assert value.atLeast(1).message("Positive argument")
  result = value * 2
  validate(result < 8).message("Bounded local")
  ret result
}`
    expect(compile(definition + '\nshow box([double(2),1,1])').source)
      .toBe(compile('show box([4,1,1])').source)
    expect(() => compile(definition + '\nshow box([double(0),1,1])')).toThrow('Positive argument')
    expect(() => compile(definition + '\nshow box([double(4),1,1])')).toThrow('Bounded local')
  })

  it('retains layout function checks after canonical parameter edits', () => {
    const source = `param size = 2 range 0..4
fn checked value: int -> int
  local = value + 1
  assert(local > 1).message("Local follows parameter")
  ret local
show box([checked(size),1,1])`
    const compiled = compile(source)
    expect(compiled.source).toBe(compile('show box([3,1,1])').source)
    const changed = structuredClone(compiled.document)
    changed.parameters[0]!.value = 0
    expect(() => compileModelGraph(changed)).toThrow('Local follows parameter')
  })

  it.each(['', '  ret\n'])('executes no-result helper checks with explicit ret=%s', end => {
    const definition = `fn requirePositive value: int
  assert(value > 0).message("Void helper failed")
${end}
fn checked value: int -> int
  requirePositive(value)
  ret value
`
    expect(compile(definition + 'show box([checked(2),1,1])').source)
      .toBe(compile('show box([2,1,1])').source)
    expect(() => compile(definition + 'show box([checked(0),1,1])')).toThrow('Void helper failed')
    expect(() => compile(definition + 'requirePositive(0)\nshow box([1,1,1])')).toThrow('Void helper failed')
  })

  it('does not leak no-result helper checks out of unselected scalar or geometry branches', () => {
    const definition = `fn guard value: int
  assert(value > 0).message("Selected helper only")
fn checked value: int -> int
  guard(value)
  ret value
fn part value: int -> Geometry
  guard(value)
  ret box([1,1,1])
`
    expect(compile(definition + 'show box([1 ? 2 : checked(0),1,1])').source)
      .toBe(compile('show box([2,1,1])').source)
    expect(compile(definition + 'show 0 ? part(0) : box([2,1,1])').source)
      .toBe(compile('show box([2,1,1])').source)
    expect(() => compile(definition + 'show box([0 ? 2 : checked(0),1,1])')).toThrow('Selected helper only')
    expect(() => compile(definition + 'show 1 ? part(0) : box([2,1,1])')).toThrow('Selected helper only')
  })

  it('executes checks only in the selected match arm and keeps its local scope', () => {
    const source = `param choice = 1 range 0..1
local = 10
size = match choice
  1 =>
    local = 2
    assert local.equalTo(2)
    ret local
  _ =>
    validate(0 > 1).message("Selected match failure")
    ret 3
assert local.equalTo(10)
show box([size,1,1])`
    expect(compile(source).source).toBe(compile('show box([2,1,1])').source)
    expect(() => compile(source.replace('choice = 1', 'choice = 0'))).toThrow('Selected match failure')
  })

  it('keeps checks on functions and match arms returning empty lists', () => {
    const definition = `fn empty value: int -> Vec<int>
  assert(value > 0).message("Empty function result")
  ret []
`
    expect(compile(definition + 'show box([empty(1).count()+1,1,1])').source)
      .toBe(compile('show box([1,1,1])').source)
    expect(() => compile(definition + 'show box([empty(0).count()+1,1,1])')).toThrow('Empty function result')
    const branch = `values = match 1
  _ =>
    assert(0 > 1).message("Empty match result")
    ret []
show box([values.count()+1,1,1])`
    expect(() => compile(branch)).toThrow('Empty match result')
  })

  it('keeps failing assertions on selected empty geometry branches', () => {
    const source = `part = match 0
  0 =>
    assert(1 > 0).message("Empty geometry branch")
    ret []
  _ => box([2,1,1])
show [part, box([1,1,1])]`
    expect(compile(source).source).toBe(compile('show box([1,1,1])').source)
    expect(() => compile(source.replace('assert(1 > 0)', 'assert(0 > 1)'))).toThrow('Empty geometry branch')
  })

  it('honors foreach continue and break before following assertions', () => {
    const source = `values = foreach value in [1,0,2,99,-1]
  continue if value == 0
  break if value == 99
  assert(value > 0).message("Positive yielded value")
  validate value.atMost(2).message("Bounded yielded value")
  yield value
assert values.sum().equalTo(3)
show box([values.sum(),1,1])`
    expect(compile(source).source).toBe(compile('show box([3,1,1])').source)
    expect(() => compile(source.replace('[1,0,2,99,-1]', '[1,0,-1,99,2]'))).toThrow('Positive yielded value')
    expect(() => compile(source.replace('[1,0,2,99,-1]', '[1,0,3,99,-1]'))).toThrow('Bounded yielded value')
  })

  it('executes assertions before a later continue or break even if no item is yielded', () => {
    for (const stop of ['continue if value == 0', 'break if value == 0']) {
      const source = `values = foreach value in [0]
  assert(value > 0).message("Before loop control")
  ${stop}
  yield value
show box([values.count()+1,1,1])`
      expect(() => compile(source)).toThrow('Before loop control')
    }
  })

  it('stops evaluating earlier assertions once foreach break terminates iteration', () => {
    const source = `values = foreach value in [1,2,-3]
  assert(value > 0).message("Before break only while active")
  break if value == 2
  yield value
show box([values.sum(),1,1])`
    expect(compile(source).source).toBe(compile('show box([1,1,1])').source)
    expect(() => compile(source.replace('break if value == 2', 'break if value == 9')))
      .toThrow('Before break only while active')
  })

  it('runs named function checks inside LINQ callbacks for every selected item', () => {
    const definition = `fn double value: int -> int
  assert(value > 0).message("LINQ item")
  ret value * 2
`
    expect(compile(definition + 'show box([[1,2].select(x => double(x)).sum(),1,1])').source)
      .toBe(compile('show box([6,1,1])').source)
    expect(() => compile(definition + 'show box([[1,0].select(x => double(x)).sum(),1,1])')).toThrow('LINQ item')
    expect(compile(definition + 'show box([[0,2].where(x => x > 0).select(x => double(x)).sum(),1,1])').source)
      .toBe(compile('show box([4,1,1])').source)
  })

  it('runs named function checks inside repeat callbacks with each instance index', () => {
    const definition = `fn part index: int -> Geometry
  assert(index < 2).message("Repeat instance")
  ret box([1,1,1]).translate([index*3,0,0])
`
    expect(compile(definition + 'show repeat(2, i => part(i))').source)
      .toBe(compile('show repeat(2, i => box([1,1,1]).translate([i*3,0,0]))').source)
    expect(() => compile(definition + 'show repeat(3, i => part(i))')).toThrow('Repeat instance')
  })

  it.each([
    'part()\nshow sphere(1)',
    'unused = part()\nshow sphere(1)',
    'fn radius -> int\n  unused = part()\n  ret 1\nshow sphere(radius())',
  ])('executes checks in discarded geometry calls: %s', usage => {
    const definition = `fn part -> Geometry
  assert(true).message("Discarded geometry call")
  ret box([2,3,4])
`
    expect(compile(definition + usage).source).toBe(compile('show sphere(1)').source)
    expect(() => compile(definition.replace('assert(true)', 'assert(false)') + usage))
      .toThrow('Discarded geometry call')
  })

  it('keeps assertions when a discarded function returns transformed geometry', () => {
    const definition = `fn part -> Geometry
  assert(true).message("Discarded transformed body")
  body = box([2,3,4])
  ret body.translate([10,0,0]).scale([1,1,2])
`
    expect(compile(definition + 'part()\nshow sphere(1)').source)
      .toBe(compile('show sphere(1)').source)
    expect(() => compile(definition.replace('assert(true)', 'assert(false)') + 'part()\nshow sphere(1)'))
      .toThrow('Discarded transformed body')
  })

  it('evaluates only the selected returned geometry branch when its result is discarded', () => {
    const definition = `fn checked -> Geometry
  assert(false).message("Discarded selected branch")
  ret box([2,3,4])
fn conditional choice: int -> Geometry
  ret choice ? checked() : box([1,1,1])
fn matched choice: int -> Geometry
  ret match choice
    1 => checked()
    _ => box([1,1,1])
`
    for (const name of ['conditional', 'matched']) {
      expect(compile(definition + `${name}(0)\nshow sphere(1)`).source)
        .toBe(compile('show sphere(1)').source)
      expect(() => compile(definition + `${name}(1)\nshow sphere(1)`))
        .toThrow('Discarded selected branch')
    }
  })

  it('runs one scalar assertion when a checked result is referenced three times', () => {
    const source = `fn checked size: int -> int
  assert(size > 0).message("One scalar call")
  ret size
fn outer size: int -> int
  result = checked(size)
  ret result
size = outer(2)
show box([size,size,size])`
    const compiled = compile(source)
    expect(compiled.source).toBe(compile('show box([2,2,2])').source)
    expect(compiled.constraint_report.filter(check => check.message === 'One scalar call')).toHaveLength(1)
    expect(compileModelGraph(compiled.document).constraint_report
      .filter(check => check.message === 'One scalar call')).toHaveLength(1)
    expect(() => compile(source.replace('outer(2)', 'outer(0)'))).toThrow('One scalar call')
  })
})

describe('ModelGraph scoped geometry assertions', () => {
  it('builds every complete example in the assertion reference', async () => {
    const guide = readFileSync('docs/languages/modelgraph-text-assertions.md', 'utf8')
    const examples = [...guide.matchAll(/```text\n([\s\S]*?)```/g)].map(match => match[1]!)
    expect(examples.length).toBeGreaterThan(0)
    for (const source of examples) {
      expect((await parseOpenSCAD(source)).meshes.length).toBeGreaterThan(0)
    }
  })

  it('measures an intermediate target independently of the shown root', async () => {
    const source = `part = box([2,3,4])
assert part.hasBodies(1).isWatertight()
assert measure(part).height.approximately(4mm).message("Intermediate height")
show part.scale([1,1,2])`
    const built = await build(source)
    expect(built.meshes).toHaveLength(1)
    expect(built.volume).toBeCloseTo(48)
    await expect(build(source.replace('approximately(4mm)', 'approximately(8mm)')))
      .rejects.toThrow('Intermediate height')
  })

  it('checks a local hidden body even when its function returns a number', async () => {
    const source = `fn checkedHeight height: length -> length
  local = box([2mm,3mm,height])
  assert local.isWatertight()
  assert measure(local).height.approximately(height).message("Hidden local height")
  ret height * 2
show box([1mm,1mm,checkedHeight(4mm)])`
    expect((await build(source)).volume).toBeCloseTo(8)
    await expect(build(source.replace('approximately(height)', 'approximately(height + 1mm)')))
      .rejects.toThrow('Hidden local height')
  })

  it('builds geometry assertion targets from discarded calls without adding them to the scene', async () => {
    const source = `fn part -> Geometry
  body = box([2,3,4])
  assert body.hasBodies(1).message("Discarded target geometry")
  ret body
part()
show box([1,1,1])`
    const built = await build(source)
    expect(built.meshes).toHaveLength(1)
    expect(built.volume).toBeCloseTo(1)
    await expect(build(source.replace('hasBodies(1)', 'hasBodies(2)')))
      .rejects.toThrow('Discarded target geometry')
  })

  it('checks function-local geometry before a later transformation', async () => {
    const source = `fn part height: length -> Geometry
  body = box([2mm,3mm,height])
  assert measure(body).height.approximately(height).message("Local body height")
  ret body.scale([1,1,2])
show part(4mm)`
    expect((await build(source)).volume).toBeCloseTo(48)
    await expect(build(source.replace('approximately(height)', 'approximately(height * 2)')))
      .rejects.toThrow('Local body height')
  })

  it('does not build or check geometry from unselected conditional and match branches', async () => {
    const definition = `fn checked value: int -> Geometry
  local = box([1,1,1])
  assert local.hasBodies(value).message("Selected geometry only")
  ret local
`
    for (const selected of [
      'show 0 ? checked(2) : box([2,1,1])',
      'show match 0\n  1 => checked(2)\n  _ => box([2,1,1])',
    ]) {
      expect((await build(definition + selected)).volume).toBeCloseTo(2)
    }
    await expect(build(definition + 'show 1 ? checked(2) : box([2,1,1])'))
      .rejects.toThrow('Selected geometry only')
    await expect(build(definition + 'show match 1\n  1 => checked(2)\n  _ => box([2,1,1])'))
      .rejects.toThrow('Selected geometry only')
  })

  it('resolves local geometry and expected dimensions for every foreach iteration', async () => {
    const source = `parts = foreach height in [1mm,2mm,3mm]
  local = box([1mm,1mm,height])
  assert measure(local).height.approximately(height).message("Loop-local height")
  yield local.translate([height*3,0,0])
show parts`
    const built = await build(source)
    expect(built.meshes).toHaveLength(3)
    expect(built.volume).toBeCloseTo(6)
    await expect(build(source.replace('approximately(height)', 'approximately(1mm)')))
      .rejects.toThrow('Loop-local height')
  })

  it('skips geometry checks after foreach continue and break', async () => {
    const source = `parts = foreach height in [1,0,2,99,-1]
  continue if height == 0
  break if height == 99
  local = box([1,1,height])
  assert measure(local).height.approximately(height * 1mm).message("Active geometry iteration")
  yield local.translate([height*3,0,0])
show parts`
    expect((await build(source)).meshes).toHaveLength(2)
    await expect(build(source.replace('approximately(height * 1mm)', 'approximately(1mm)')))
      .rejects.toThrow('Active geometry iteration')
  })

  it('keeps geometry assertions before break on the terminating item without checking later items', async () => {
    const source = `parts = foreach expected in [1,1,2]
  local = box([1,1,1])
  assert local.hasBodies(expected).message("Geometry before break")
  break if expected == 1
  yield local
show [parts, box([1,1,1])]`
    expect((await build(source)).volume).toBeCloseTo(1)
    await expect(build(source.replace('[1,1,2]', '[2,1,1]'))).rejects.toThrow('Geometry before break')
  })

  it('carries geometry checks through named functions inside repeat and LINQ callbacks', async () => {
    const definition = `fn part index: int -> Geometry
  local = box([1,1,index+1])
  assert measure(local).height.approximately((index+1)*1mm).message("Callback geometry")
  ret local.translate([index*3,0,0])
`
    for (const [output, meshes] of [
      ['show repeat(2, i => part(i))', 1],
      ['show [0,1].select(i => part(i))', 2],
    ] as const) {
      const built = await build(definition + output)
      expect(built.meshes).toHaveLength(meshes)
      expect(built.volume).toBeCloseTo(3)
      await expect(build(definition.replace('approximately((index+1)*1mm)', 'approximately(1mm)') + output))
        .rejects.toThrow('Callback geometry')
    }
  })

  it('evaluates hidden geometry checks exactly once per nested scalar call and repeat instance', () => {
    const source = `fn checked index: int -> int
  hidden = box([index+1,1,1])
  assert hidden.hasBodies(1).message("One hidden body per instance")
  ret index + 1
fn outer index: int -> int
  value = checked(index)
  ret value
show repeat(100, i => box([outer(i),1,1]).translate([i*101,0,0]))`
    const compiled = compile(source)
    expect(compiled.geometry_assertions).toHaveLength(100)
    expect(compiled.geometry_assertions.every(check => check.message === 'One hidden body per instance')).toBe(true)
    expect(new Set(compiled.geometry_assertions.map(check => check.source)).size).toBe(100)
    expect(compileModelGraph(compiled.document).geometry_assertions).toHaveLength(100)
  })
})
