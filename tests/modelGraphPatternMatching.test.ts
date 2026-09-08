import { describe, expect, it } from 'vitest'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { formatCode } from '../src/services/codeFormat'

const header = '// @modelgraph-text/1\n'
const compile = (source: string) => compileModelGraphText(header + source)

function expectValue(value: string, arms: string, expected: string | number) {
  const source = `result = match ${value}\n${arms}\nvalidate result.equalTo(${expected})\nshow box([1,1,1])`
  expect(compile(source).constraint_report.every(report => report.passed)).toBe(true)
}

describe('ModelGraph structural pattern matching', () => {
  it.each([
    ['7', '7', 1],
    ['7', '8', 0],
    ['-3', '-3', 1],
    ['2.5', '2.5', 1],
    ['"hook"', '"hook"', 1],
    ['"Hook"', '"hook"', 0],
    ['""', '""', 1],
    ['"a\\\"b"', '"a\\\"b"', 1],
    ['10mm', '1cm', 1],
    ['1m', '1000mm', 1],
    ['90deg', '90deg', 1],
    ['1rad', '1rad', 1],
    ['10mm', '10', 0],
    ['10mm', '10deg', 0],
    ['"7"', '7', 0],
    ['true', 'true', 1],
    ['false', 'true', 0],
  ])('compares literal %s against %s', (value, pattern, expected) => {
    expectValue(value, `  ${pattern} => 1\n  _ => 0`, expected)
  })

  it('binds a bare name even if an outer variable has the same name', () => {
    const source = `value = 99
result = match 4
  value => value * 2
validate result.equalTo(8)
validate value.equalTo(99)
show sphere(result)`
    expect(compile(source).constraint_report.every(report => report.passed)).toBe(true)
  })

  it('does not require a wildcard when a concrete arm matches', () => {
    expectValue('3', '  3 => 8', 8)
  })

  it.each([
    ['1', '1..3', 1],
    ['3', '1..3', 1],
    ['4', '1..3', 0],
    ['1', '1..<3', 1],
    ['3', '1..<3', 0],
    ['-2', '-3..-1', 1],
    ['1.5', '1..<2', 1],
    ['1cm', '5mm..15mm', 1],
    ['1cm', '5mm..<1cm', 0],
    ['90deg', '0deg..180deg', 1],
    ['90mm', '0deg..180deg', 0],
    ['"2"', '1..3', 0],
  ])('applies range %s in %s', (value, pattern, expected) => {
    expectValue(value, `  ${pattern} => 1\n  _ => 0`, expected)
  })

  it('binds a range match for use in a guard and result', () => {
    expectValue('7', '  n @ 1..10 where n % 2 == 1 => n * 2\n  _ => 0', 14)
  })

  it.each([
    ['[]', '[]', 1],
    ['[1]', '[]', 0],
    ['[1,2]', '[1,2]', 1],
    ['[1,2,3]', '[1,2]', 0],
    ['[1]', '[1,2]', 0],
    ['[1,2,3,4]', '[1, .., 4]', 1],
    ['[1,4]', '[1, .., 4]', 1],
    ['[1]', '[1, .., 1]', 0],
    ['[]', '[..]', 1],
    ['3', '[..]', 0],
  ])('checks list shape %s against %s', (value, pattern, expected) => {
    expectValue(value, `  ${pattern} => 1\n  _ => 0`, expected)
  })

  it.each([
    ['[1,2,3,4]', '[first, ..middle, last]', 'first + middle.sum() + last', 10],
    ['[1,4]', '[first, ..middle, last]', 'middle.count()', 0],
    ['[]', '[..items]', 'items.count()', 0],
    ['[2,3,4]', '[..prefix, last]', 'prefix.sum() * last', 20],
    ['[2,3,4]', '[first, ..tail]', 'first * tail.sum()', 14],
    ['[[1,2], [3,4]]', '[[x, _], [_, y]]', 'x + y', 5],
    ['[1mm,2mm,3mm]', '[first, ..tail]', 'first + tail.sum()', '6mm'],
  ])('destructures %s with %s', (value, pattern, result, expected) => {
    expectValue(value, `  ${pattern} => ${result}`, expected)
  })

  it.each([
    ['{x:2,y:3}', '{x}', 'x', 2],
    ['{x:2,y:3}', '{x: width, y: height}', 'width * height', 6],
    ['{x:2,y:3,z:4}', '{x, ..}', 'x', 2],
    ['{x:2}', '{x, !}', 'x', 2],
    ['{x:2,y:3}', '{x, !}', 'x', 0],
    ['{}', '{!}', '1', 1],
    ['{x:2}', '{!}', '1', 0],
    ['{x:2}', '{}', '1', 1],
    ['{x:2}', '{..}', '1', 1],
    ['{y:3}', '{x}', 'x', 0],
    ['[2,3]', '{x}', 'x', 0],
    ['{kind:"circle",radius:4}', '{kind: "circle", radius: r}', 'r * 2', 8],
    ['{kind:"box",size:[2,3]}', '{kind: "box", size: [width, height]}', 'width * height', 6],
    ['{position:{x:3,y:4},tag:"p"}', '{position: {x, y}}', 'x + y', 7],
  ])('destructures record %s with %s', (value, pattern, result, expected) => {
    expectValue(value, `  ${pattern} => ${result}\n  _ => 0`, expected)
  })

  it('retains the whole value with an alias while destructuring its fields', () => {
    expectValue('{x:3,y:4}', '  point @ {x} => x + point.y', 7)
    expectValue('[2,3,4]', '  all @ [head, ..tail] => all.count() + head + tail.sum()', 12)
  })

  it.each([
    ['0', '0 | 2 | 4', '1', 1],
    ['3', '0 | 2 | 4', '1', 0],
    ['"box"', '"sphere" | "box"', '1', 1],
    ['[0,7]', '[0, n] | [n, 0]', 'n', 7],
    ['[7,0]', '[0, n] | [n, 0]', 'n', 7],
    ['{left:8}', '{left: n} | {right: n}', 'n', 8],
    ['{right:9}', '{left: n} | {right: n}', 'n', 9],
  ])('evaluates alternatives for %s', (value, pattern, result, expected) => {
    expectValue(value, `  ${pattern} => ${result}\n  _ => 0`, expected)
  })

  it.each([
    ['2', 'int(n)', 'n', 2],
    ['2.5', 'int(n)', 'n', 0],
    ['2mm', 'int(n)', '1', 0],
    ['2', 'f64(n)', 'n', 2],
    ['2.5', 'f64(n)', 'n', 2.5],
    ['2.5', 'f32(n)', 'n', 2.5],
    ['"x"', 'str(s)', 's == "x"', 1],
    ['2', 'str(s)', '1', 0],
    ['2cm', 'length(n)', 'n / 1mm', 20],
    ['90deg', 'angle(n)', 'n / 1deg', 90],
    ['90deg', 'length(n)', '1', 0],
    ['2mm', 'f64(n)', '1', 0],
    ['[2,3]', 'list(xs)', 'xs.sum()', 5],
    ['{x:7}', 'record(r)', 'r.x', 7],
    ['[]', 'record(r)', '1', 0],
    ['[1,2,3]', 'list([first, ..tail])', 'first + tail.sum()', 6],
    ['{x:2,y:3}', 'record({x, y})', 'x + y', 5],
    ['4', 'int(n @ 1..5)', 'n', 4],
  ])('checks builtin type pattern %s as %s', (value, pattern, result, expected) => {
    expectValue(value, `  ${pattern} => ${result}\n  _ => 0`, expected)
  })

  it('tries later arms after a false guard', () => {
    expectValue('[2,8]', '  [a, b] where a > b => 100\n  [a, b] where b > a => b-a\n  _ => 0', 6)
  })

  it('never evaluates guards for a failed pattern or later branches after a match', () => {
    expectValue('2', '  1 where 1 / 0 => 100\n  2 => 8\n  _ where 1 / 0 => 1 / 0', 8)
    expectValue('[0,4]', '  [x, y] where x != 0 && y/x > 1 => 100\n  _ => 8', 8)
    expectValue('{kind:"box"}', '  {radius: r} where r > 0 && 1/0 > 0 => r\n  _ => 8', 8)
    expectValue('{kind:"box"}', '  {radius: r} => 1/0\n  _ => 8', 8)
  })

  it('tries the first matching alternative once before evaluating its arm guard', () => {
    // Both alternatives match. The first binds n=0: its false guard continues
    // to the next arm rather than retrying the same arm with n=1.
    expectValue('[0,1]', '  [n, _] | [_, n] where n > 0 => n\n  _ => 9', 9)
  })

  it('does not leak partially matched bindings into later arms', () => {
    const source = `n = 10
result = match [7,0]
  [n, 1] => n
  _ => n
validate result.equalTo(10)
show sphere(1)`
    expect(compile(source).constraint_report.every(report => report.passed)).toBe(true)
  })

  it('supports nested matches and scopes names independently', () => {
    expectValue('[3,4]', `  [x, y] => match {x:y, y:x}
    {x, y} => x * 10 + y`, 43)
  })

  it('evaluates branch-local assignments followed by ret', () => {
    const source = `scale = 10
result = match [2,3]
  [width, height] =>
    area = width * height
    scale = 2
    ret area * scale
  _ => 0
validate result.equalTo(12)
validate scale.equalTo(10)
show sphere(result)`
    expect(compile(source).constraint_report.every(report => report.passed)).toBe(true)
    expect(compile(formatCode(source)).source).toBe(compile(source).source)
  })

  it('returns a nested match from a branch block', () => {
    expectValue('[3,4]', `  [x, y] =>
    total = x + y
    ret match total
      n @ 1..10 => n * 2
      _ => 0`, 14)
  })

  it('does not execute an unselected branch block', () => {
    expectValue('2', `  1 =>
    impossible = 1/0
    ret impossible
  2 => 7`, 7)
  })

  it('uses match in function returns, foreach bodies and LINQ projections', () => {
    const source = `fn weight item: int -> int
  ret match item
    0 => 1
    n @ 1..3 => n * 2
    _ => 0
values = foreach x in 0..<5
  value = match x
    4 => 10
    n => weight(n)
  yield value
projected = values.select(x => match x
  10 => 20
  n => n)
validate projected.sum().equalTo(33)
show sphere(1)`
    expect(compile(source).constraint_report.every(report => report.passed)).toBe(true)
    expect(compile(formatCode(source)).source).toBe(compile(source).source)
  })

  it.each([0, 1, 2])('recomputes guarded matching for parameter %s', part => {
    const source = `param part = ${part} range 0..2
result = match part
  n where n > 0 => n + 2
  _ => 1
validate result.equalTo(${part > 0 ? part + 2 : 1})
show sphere(result)`
    expect(compile(source).constraint_report.every(report => report.passed)).toBe(true)
  })

  it('emits the same geometry as explicit record field access and selection', async () => {
    const source = `items = [{kind:"box",size:[2,3,4],x:0},{kind:"sphere",radius:2,x:10}]
parts = foreach item in items
  yield match item
    {kind: "box", size: [w, d, h], x} => box([w,d,h]).translate([x,0,0])
    {kind: "sphere", radius: r, x} where r > 0 => sphere(r).translate([x,0,0])
    _ => box([1,1,1])
show parts`
    const explicit = 'show [box([2,3,4]).translate([0,0,0]), sphere(2).translate([10,0,0])]'
    expect(compile(source).source).toBe(compile(explicit).source)
    expect(compile(formatCode(source)).source).toBe(compile(source).source)
    expect((await parseOpenSCAD(header + source)).meshes).toHaveLength(2)
  })

  it('supports geometry branch blocks and an empty geometry fallback', async () => {
    const source = `items = [{radius:0,x:0},{radius:2,x:10}]
parts = foreach item in items
  yield match item
    {radius: r, x} where r > 0 =>
      diameter = r * 2
      ret box([diameter,2,3]).translate([x,0,0])
    _ => []
show parts`
    expect(compile(source).source).toBe(compile('show box([4,2,3]).translate([10,0,0])').source)
    expect(compile(formatCode(source)).source).toBe(compile(source).source)
    expect((await parseOpenSCAD(header + source)).meshes).toHaveLength(1)
  })

  it.each([
    ['str', `fn label input: str -> str
  ret match input
    "hook" => "selected"
    other => other
input = match 1
  1 => "hook"
  _ => "box"
result = label(input)
validate (result == "selected").equalTo(1)`],
    ['Vec<int>', `fn keep input: Vec<int> -> Vec<int>
  ret match input
    [] => [0]
    list(values) => values
input = match 1
  1 => [2,3,4]
  _ => [0]
result = keep(input)
validate result.sum().equalTo(9)`],
    ['int', `fn twice input: int -> int
  ret match input
    int(n) => n * 2
input = match 1
  1 => 4
  _ => 0
result = twice(input)
validate result.equalTo(8)`],
    ['nominal record', `struct Pair {x: int, y: int}
fn swap input: Pair -> Pair
  ret match input
    {x, y} => Pair {x:y, y:x}
input = match 1
  1 => Pair {x:2, y:3}
  _ => Pair {x:0, y:0}
result = swap(input)
validate result.x.equalTo(3)
validate result.y.equalTo(2)`],
  ])('preserves typed %s function arguments and returns through match', (_type, source) => {
    expect(compile(source + '\nshow sphere(1)').constraint_report.every(report => report.passed)).toBe(true)
  })

  it('infers generic types through protected dynamic vector results', () => {
    const source = `fn vector x: int -> Vec<int>
  ret match x
    _ => [1,2,3]
fn identity[T] x: T -> T
  ret x
v = identity(vector(1))
show box(v)`
    expect(compile(source).source).toBe(compile('show box([1,2,3])').source)
  })

  it.each([
    ['unused argument with text result', `fn accept input: str -> str
  ret "ok"
input = match 1
  1 => 4
  _ => "text"
result = accept(input)
validate (result == "ok").equalTo(1)
show sphere(1)`],
    ['str argument', `fn accept input: str -> int
  ret 1
input = match 1
  1 => 4
  _ => "text"
show sphere(accept(input))`],
    ['Vec<int> element', `fn accept input: Vec<int> -> int
  ret input.count()
input = match 1
  1 => [2,3.5]
  _ => [2,3]
show sphere(accept(input))`],
    ['int return', `fn choose input: int -> int
  ret match input
    1 => 2.5
    _ => 2
show sphere(choose(1))`],
    ['record field', `struct Pair {x: int, y: int}
fn choose input: int -> Pair
  ret match input
    1 => Pair {x:2, y:3.5}
    _ => Pair {x:2, y:3}
show sphere(choose(1).x)`],
  ])('enforces the selected match value at a typed %s boundary', (_type, source) => {
    expect(() => compile(source)).toThrow()
  })

  it.each([
    ['box', '[2,3,4]', '[5,6,7]', 'box(vector)', 'box([2,3,4])'],
    ['rectangle', '[2,3]', '[5,6]', 'rectangle(vector).extrude(1)', 'rectangle([2,3]).extrude(1)'],
    ['translate', '[2mm,3mm,4mm]', '[5mm,6mm,7mm]', 'box([1,1,1]).translate(vector)', 'box([1,1,1]).translate([2mm,3mm,4mm])'],
    ['rotate', '[90deg,0deg,45deg]', '[0deg,45deg,90deg]', 'box([1,2,3]).rotate(vector)', 'box([1,2,3]).rotate([90deg,0deg,45deg])'],
    ['scale', '[2,3,4]', '[5,6,7]', 'box([1,2,3]).scale(vector)', 'box([1,2,3]).scale([2,3,4])'],
  ])('passes dynamically selected vectors to %s', (_operation, selected, other, call, explicit) => {
    const source = `param choice = 1 range 0..1
vector = match choice
  1 => ${selected}
  _ => ${other}
show ${call}`
    expect(compile(source).source).toBe(compile(`show ${explicit}`).source)
  })

  it.each([0, 1])('accepts a parenthesis-free ternary guard for value %s', value => {
    expectValue(String(value), '  n where n > 0 ? true : false => 2\n  _ => 1', value > 0 ? 2 : 1)
  })

  it.each([0, 1])('ends a nested match before the enclosing ternary colon for condition %s', condition => {
    const source = `result = ${condition} ? match 1
  1 => 7
  _ => 8 : 9
validate result.equalTo(${condition ? 7 : 9})
show sphere(result)`
    expect(compile(source).constraint_report.every(report => report.passed)).toBe(true)
  })

  it('applies a dedented fluent chain to the complete match result', () => {
    const source = `values = match 0
  0 => [1,2]
  _ => [3,4]
.select(x => x * 2)
.sum()
validate values.equalTo(6)
show sphere(values)`
    expect(compile(source).source).toContain('sphere(r=6')
    expect(compile(source).constraint_report.every(report => report.passed)).toBe(true)
  })

  it.each([0, 1])('allows an entirely empty geometry match in show for branch %s', async branch => {
    const source = `show match ${branch}
  0 => []
  _ => []`
    expect(() => compile(source)).not.toThrow()
    expect((await parseOpenSCAD(header + source)).meshes).toHaveLength(0)
  })

  it.each([
    ['no matching literal', 'result = match 1\n  2 => 3\nshow sphere(result)'],
    ['all guards false', 'result = match 1\n  n where n > 10 => n\nshow sphere(result)'],
    ['incompatible range endpoints', 'result = match 1mm\n  0mm..2deg => 1\n  _ => 0\nshow sphere(result)'],
    ['reversed range', 'result = match 2\n  3..1 => 1\n  _ => 0\nshow sphere(result)'],
    ['multiple list rests', 'result = match [1,2]\n  [..left, ..right] => 1\nshow sphere(result)'],
    ['duplicate list binding', 'result = match [1,2]\n  [x, x] => x\nshow sphere(result)'],
    ['duplicate record binding', 'result = match {x:1,y:2}\n  {x:n,y:n} => n\nshow sphere(result)'],
    ['duplicate alias binding', 'result = match [1]\n  x @ [x] => x\nshow sphere(result)'],
    ['unequal alternative bindings', 'result = match [1]\n  [x] | [y] => 1\nshow sphere(result)'],
    ['missing alternative binding', 'result = match [1]\n  [x] | [] => 1\nshow sphere(result)'],
    ['unknown type pattern', 'result = match 1\n  unknown(n) => n\nshow sphere(result)'],
    ['extra type arguments', 'result = match 1\n  int(a,b) => a\nshow sphere(result)'],
    ['binding escaping its match', 'result = match 1\n  hidden => hidden\nshow sphere(hidden)'],
    ['branch local escaping its match', 'result = match 1\n  _ =>\n    hidden = 2\n    ret hidden\nshow sphere(hidden)'],
    ['bad arm indentation', 'result = match 1\n  1 => 1\n   _ => 2\nshow sphere(result)'],
  ])('rejects %s', (_label, source) => {
    expect(() => compile(source)).toThrow()
  })
})
