import { expect, it } from 'vitest'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { formatCode } from '../src/services/codeFormat'

function check(expression: string, expected: number) {
  return compileModelGraphText(`// @modelgraph-text/1\nresult = ${expression}\nvalidate result.equalTo(${expected})\nshow box([1,1,1])`)
}
it.each([
  ['(0..<10).where(x => x % 2 == 0).select(x => x * 3).sum()', 60],
  ['[3,1,2].orderBy(x => x).first()',1],
  ['[4,5,6].select((x,i) => x + i).sum()',18],
  ['[4,5,6].where((x,i) => i > 0).sum()',11],
  ['[4,5,6].takeWhile((x,i) => i < 2).sum()',9],
  ['[1,2,3].scan(0, (acc,x) => acc + x).sum()',10],
  ['[].defaultIfEmpty(8).first()',8],
  ['[1,2].join([2,2,3], x => x, y => y, (x,y) => x+y).sum()',8],
  ['[1,2].groupJoin([2,2,3], x => x, y => y, (x,ys) => ys.count()).sum()',2],
  ['[{tag:"a",x:3},{tag:"b",x:5}].where(p => p.tag == "a").first().x',3],
  ['[3,1,2].orderByDescending(x => x).first()',3],
  ['[1,2,3].aggregate(10, (acc, x) => acc + x)',16],
  ['[1,2].selectMany(x => [x,x]).count()',4],
  ['[1,2,3].take(2).sum()',3],
  ['[1,2,3].skip(2).first()',3],
  ['[1,2,1,3].takeWhile(x => x < 3).count()',3],
  ['[1,2,1,3].skipWhile(x => x < 2).count()',3],
  ['[1,2,3,4,5].chunk(2).select(x => x.count()).sum()',5],
  ['[1,2,3,4].window(2).select(x => x.sum()).sum()',15],
  ['[1,2,3].reverse().first()',3],
  ['[1,1,2].distinct().count()',2],
  ['[1,2,3].distinctBy(x => x % 2).count()',2],
  ['[1,2,3].concat([4]).append(5).prepend(0).sum()',15],
  ['[1,2].union([2,3]).sum()',6],
  ['[1,2,2,3].intersect([2,3]).sum()',5],
  ['[1,2,2,3].except([2]).sum()',4],
  ['[[1,2],[3]].flatten().sum()',6],
  ['[1,2].contains(2)',1],
  ['[1,2].any(x => x > 1)',1],
  ['[1,2].all(x => x > 0)',1],
  ['[].all(x => x > 0)',1],
  ['[].any()',0],
  ['[].sum()',0],
  ['[].aggregate(17, (acc, x) => acc + x)',17],
  ['[].firstOrDefault(7)',7],
  ['[].lastOrDefault(9)',9],
  ['[1,2,3].first(x => x > 1)',2],
  ['[1,2,3].last(x => x < 3)',2],
  ['[1,2,3].single(x => x == 2)',2],
  ['[1,2,3].count(x => x > 1)',2],
  ['[1,2,3].average()',2],
  ['[1,2,3].min(x => x * 2)',2],
  ['[1,2,3].max()',3],
  ['[1,2,3].elementAt(1)',2],
  ['[1,2].zip([3,4]).select(x => at(x,0) + at(x,1)).sum()',10],
  ['[3,5].enumerate().select(x => at(x,0) * at(x,1)).sum()',5],
  ['[{x: 2, y: 3}, {x: 1, y: 8}].orderBy(p => p.x).first().y',8],
  ['[{x:1,y:2},{x:0,y:9},{x:1,y:3}].orderBy(p => p.x).thenByDescending(p => p.y).skip(1).first().y',3],
  ['[1,2,3,4].groupBy(x => x % 2).select(g => g.items.sum()).sum()',10],
  ['[1,2,3,4].groupBy(x => x % 2).first().key',1],
  ['[{tag:"b",x:2},{tag:"a",x:1}].orderBy(p => p.tag).first().x',1],
])('evaluates %s', (expression, expected) => { expect(check(expression,expected).constraint_report.every(c=>c.passed)).toBe(true) })

it.each([
  '[].first()', '[].last()', '[].single()', '[].average()', '[].min()',
  '[1,2].single()', '[1].take(-1)', '[1].skip(0.5)', '[1].chunk(0)',
  '[1].thenBy(x => x)', '[1].where((x,y,z) => x)',
  '[1mm,2deg].orderBy(x => x)', '[1].zip([2,3])',
  '(0..<256).concat([1]).count()', '[1].all()',
])('rejects invalid query %s', expression => {expect(()=>check(expression,0)).toThrow()})

it('foreach has lexical locals, ordered continue/break and nested blocks',()=>{
  const body=`// @modelgraph-text/1
values = foreach i in 0..<10
  x = i * 2
  continue if x == 2
  break if x >= 8
  yield x
validate values.sum().equalTo(10)
parts = foreach x in values
  yield foreach y in 0..<2
    r = y + 1
    yield sphere(r).translate([x * 5,y * 5,0])
show parts`
  expect(compileModelGraphText(body).source).toContain('sphere')
  expect(compileModelGraphText(formatCode(body)).source).toBe(compileModelGraphText(body).source)
})
it('supports foreach header filters, destructuring, expression form and dynamic parameters',()=>{
  const source=`// @modelgraph-text/1
param n: 3 range 1..8
parts = foreach (i,x) in enumerate(0..<n) let size = x + 1 where i > 0 => box([size,1,1]).translate([i*4,0,0])
show parts`
  expect(compileModelGraphText(source).source).not.toBe(compileModelGraphText(source.replace('n: 3','n: 4')).source)
})
it('builds foreach and select into real separate meshes',async()=>{
  const source=`// @modelgraph-text/1
foreach i in (0..<4).where(x => x % 2 == 0)
  yield box([1,1,1]).translate([i*3,0,0])`
  expect((await parseOpenSCAD(source)).meshes).toHaveLength(2)
  expect((await parseOpenSCAD('// @modelgraph-text/1\nshow (0..<3).select(i => box([1,1,1]).translate([i*3,0,0]))')).meshes).toHaveLength(3)
})
it('short circuits terminal predicates and keeps unit-aware sums',()=>{
  check('[0,1].any(x => x == 0 ? 1 : 1 / 0)',1)
  check('[0,1].all(x => x == 0 ? 0 : 1 / 0)',0)
  expect(compileModelGraphText('// @modelgraph-text/1\nr = [1mm,2mm].sum()\nshow sphere(r)').source).toContain('3')
})

it('indexed geometry selection binds value and index separately',async()=>{
  const source='// @modelgraph-text/1\nshow [2,3].select((r,i) => box([r,1,1]).translate([i*10,0,0]))'
  const compiled=compileModelGraphText(source)
  expect(compiled.source).toContain('10')
  const built=await parseOpenSCAD(source)
  expect(built.meshes).toHaveLength(2)
})
it('does not leak foreach locals and rejects malformed loop bodies',()=>{
  for(const body of [
    'values = foreach x in [1]\n  y = x\nshow box([1,1,1])',
    'values = foreach x in [1]\n  yield x\nshow sphere(x)',
    'values = foreach (x,x) in [[1,2]] => x\nshow sphere(1)',
  ]) expect(()=>compileModelGraphText('// @modelgraph-text/1\n'+body)).toThrow()
})
it('foreach works inside functions and preserves filter before break ordering',()=>{
  const c=compileModelGraphText(`// @modelgraph-text/1
fn row n: int -> Geometry
  parts = foreach i in 0..<n
    continue if i == 2
    break if i == 2
    yield box([1,1,1]).translate([i*3,0,0])
  ret parts
show row(4)`)
  expect(c.source).toContain('9')
})
it('supports empty string keys, geometry lists per iteration, and the documented example', async()=>{
  check('[{tag:"",x:3}].where(p => p.tag == "").first().x',3)
  const multi='// @modelgraph-text/1\nshow foreach i in 0..<2 => [box([1,1,1]).translate([i*10,0,0]), box([1,1,1]).translate([i*10,3,0])]'
  expect((await parseOpenSCAD(multi)).meshes).toHaveLength(4)
  const {readFileSync}=await import('node:fs')
  const source=readFileSync('examples/modelgraph-text/foreach-linq.mg','utf8')
  expect((await parseOpenSCAD(source)).meshes).toHaveLength(12)
  expect(compileModelGraphText(formatCode(source)).source).toBe(compileModelGraphText(source).source)
})

it('combines predicates with logical precedence and short circuiting',()=>{
  check('(0..<8).where(x => x == 0 || x > 2 && x < 5).sum()',7)
  check('[0,1].where(x => !(x == 0) && 10/x > 1).sum()',1)
})
