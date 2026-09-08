import { expect, it } from 'vitest'
import { compileModelGraphText, modelGraphTextControls } from '../src/services/modelGraphText'
import { compileModelGraph, ModelGraphError } from '../src/services/modelGraph'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { checkModelGraphGeometry } from '../src/services/modelGraphChecks'
const header = '// @modelgraph-text/1\n'
const body = 'body = box([2mm,3mm,4mm])\nshow body\n'
it('collects all scalar violations and retains editable controls', () => {
 const source = header + 'param gap = 2mm range 0mm..5mm\n' + body + 'validate gap. between(0mm,1mm).message("Зазор")\nvalidate gap. lessThan(1mm).message("Too large")'
 const controls = modelGraphTextControls(source)
 expect(controls.parameters).toHaveLength(1)
 expect(controls.errors).toEqual(['Зазор','Too large'])
 try { compileModelGraphText(source); throw new Error('should fail') } catch(e) {
  expect(e).toBeInstanceOf(ModelGraphError)
  expect((e as ModelGraphError).details).toMatchObject([{status:'passed'}, {status:'failed'}, {status:'failed'}])
 }
})
it('handles strict boundaries, mixed units, messages and chained scalar assertions', () => {
 const c = compileModelGraphText(header + body + 'assert 10mm. equalTo(1cm).atLeast(10mm).atMost(10mm).approximately(10.01mm, tolerance: 0.02mm).message("a \\"quote\\"")')
 expect(c.constraint_report.every(r=>r.passed)).toBe(true)
 expect(()=>compileModelGraphText(header+body+'validate 1mm. greaterThan(1mm)')).toThrow()
 expect(()=>compileModelGraphText(header+body+'validate 1mm. equalTo(1deg)')).toThrow()
 expect(()=>compileModelGraphText(header+body+'validate 1mm. approximately(1mm, tolerance: -1mm)')).toThrow()
})
it('preserves checks in canonical JSON and measures actual transformed geometry', async () => {
 const source = header + 'body = box([2mm,3mm,4mm]).translate([10mm,20mm,30mm])\nshow body\nassert body. hasBodies(1).isWatertight().hasNoDegenerateTriangles()\nassert measure(body).height. approximately(4mm, tolerance: 0.001mm)'
 const c = compileModelGraph(compileModelGraphText(source).document)
 const built = await parseOpenSCAD(source)
 const checks = checkModelGraphGeometry(c.geometry_assertions, built.meshes)
 expect(checks.map(c=>c.status)).toEqual(['passed','passed','passed','passed'])
 expect(checks.at(-1)?.actual).toBeCloseTo(4)
})
it('blocks browser builds on failed geometry assertions',async()=>{
 await expect(parseOpenSCAD(header+body+'assert body. hasBodies(2)')).rejects.toThrow('Geometry assertions')
})
it('counts disconnected components inside a boolean mesh',async()=>{
 const source=header+'body = union(box([1mm,1mm,1mm]),box([1mm,1mm,1mm]).translate([5mm,0,0]))\nshow body\nassert body. hasBodies(2)'
 await expect(parseOpenSCAD(source)).resolves.toHaveProperty('meshes')
})
it.each(['assert body. nonexistent()','assert body. isWatertight(1)','assert measure(body).volume. approximately(1mm)','validate 1mm. equalTo(1mm).message(2)','assert body. hasBodies(-1)'])('rejects unsupported or malformed checks: %s',s=>{
 expect(()=>compileModelGraphText(header+body+s)).toThrow()
})
it('rejects non-root geometry checks',()=>{
 expect(()=>compileModelGraphText(header+'a = sphere(1mm)\nbody = a.translate([1mm,0,0])\nshow body\nassert a. isWatertight()')).toThrow('shown root')
})
it('returns unknown for empty dimensions rather than passing',()=>{
 expect(checkModelGraphGeometry([{id:'g1',check:'height',expected:0,tolerance:0,message:'Height'}],[])[0]?.status).toBe('unknown')
})
it('supports checks on arithmetic and directly transformed geometry',()=>{
 const c=compileModelGraphText(header+'param width: 4 range 1..10\nvalidate (2 * width).atLeast(4).message("width")\nshow sphere(2)\nassert (1 + 2).equalTo(3)')
 expect(c.constraint_report.every(r=>r.passed)).toBe(true)
})
it('rejects the removed pipe syntax',()=>{
 expect(()=>compileModelGraphText(header+'show sphere(2) |> translate([1,0,0])')).toThrow('Use .method')
})
it('accepts method chains continued on the next line',()=>{
 expect(compileModelGraphText(header+'show sphere(2)\n  .translate([1,0,0])').source).toContain('translate([1,0,0])')
})
