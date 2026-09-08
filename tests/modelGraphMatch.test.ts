import {expect,it} from 'vitest'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {formatCode} from '../src/services/codeFormat'
const compile=(s:string)=>compileModelGraphText('// @modelgraph-text/1\n'+s)
it.each([0,1,2])('matches geometry part %s',async part=>{
 const source=`param part = ${part} range 0..2
body = box([2,3,4])
hook = sphere(1)
show match part
  2 => hook
  1 => body
  _ => [body,hook]`
 const equivalent=source.slice(0,source.indexOf('show match'))+'show part == 2 ? hook : part == 1 ? body : [body,hook]'
 expect(compile(source).source).toBe(compile(equivalent).source)
 expect(compile(formatCode(source)).source).toBe(compile(source).source)
 expect((await parseOpenSCAD('// @modelgraph-text/1\n'+source)).meshes.length).toBe(part===0?2:1)
})
it('supports numeric units, strings, nested matches, functions and lazy branches',()=>{
 expect(compile('r = match "box"\n  "box" => 3\n  _ => 1/0\nshow sphere(r)').source).toContain('sphere(r=3')
 expect(compile('r = match -2mm\n  -2mm => 3mm\n  _ => 4mm\nshow sphere(r)').source).toContain('sphere(r=3')
 expect(compile('fn radius n: int -> f64\n  ret match n\n    1 => match n\n      1 => 3\n      _ => 4\n    _ => 5\nshow sphere(radius(1))').source).toContain('sphere(r=3')
 expect(compile('r = match 1\n  1 =>\n    2 + 3\n  _ => 6\nnext = 2\nshow sphere(r+next)').source).toContain('sphere(r=7')
})
it.each([
 'r = match 1\n  _ => 2\n  1 => 3\nshow sphere(r)',
 'r = match 1\n  1 => 2\n  1 => 3\n  _ => 4\nshow sphere(r)',
 'r = match 1\n  x => 2\n  _ => 3\nshow sphere(r)',
 'r = match 1\n  1 => 2\n   _ => 3\nshow sphere(r)',
 'r = match 1\n  1 2\n  _ => 3\nshow sphere(r)',
])('rejects malformed match %s',s=>expect(()=>compile(s)).toThrow())
