import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'
import type { NurbsCurve } from '../src/kernel/nurbsCurve'

const cases: [string, string, number, number][] = [
  ['rounded_rectangle', 'center:[0,0,0],width:10mm,height:6mm,radius:1mm', 10, 6],
  ['capsule', 'center:[0,0,0],straight:4mm,radius:2mm', 8, 4],
  ['keyway', 'mouth:[0,0,0],width:3mm,depth:2mm', 3, 2],
  ['t_slot', 'mouth:[0,0,0],neck_width:3mm,flange_width:8mm,neck_depth:2mm,total_depth:5mm', 8, 5],
  ['dovetail', 'mouth:[0,0,0],mouth_width:3mm,base_width:7mm,depth:4mm', 7, 4],
  ['poly_v_belt', 'center:[0,0,0],ribs:3,pitch:4mm,back_thickness:1mm,rib_height:2mm', 12, 3],
  ['toothed_belt','center:[0,0,0],teeth:3,pitch:5mm,base_width:4mm,tip_width:2mm,back_thickness:1mm,tooth_height:3mm',15,4],
  ['o_ring_groove','mouth:[0,0,0],width:6mm,depth:3mm,radius:1mm',6,3],
  ['square_fastener','center:[0,0,0],across_flats:6mm',6,6],
  ['hex_fastener','center:[0,0,0],across_flats:6mm',12/Math.sqrt(3),6],
]
function compileProfile(op: string, args: string) {
  return compileRushFrontend(`// @rush/1\np = ${op}_profile(${args})\nshow p.surface_extrude(vector:[0,0,2mm]).tessellate(24,2)`).document
}
it.each(cases)('builds the %s outline with its authored dimensions', async (name, args, width, height) => {
  const graph = compileProfile(name, args)
  const built = await buildOwnNurbsAsync(graph, { action:'build' })
  const curve = built.report.definitions[graph.nodes[0]!.id] as NurbsCurve
  expect(curve.controlPoints[0]).toEqual(curve.controlPoints.at(-1))
  for (const [axis, expected] of [width,height].entries()) {
    const coords = curve.controlPoints.map(p => p[axis]!)
    expect(Math.max(...coords)-Math.min(...coords)).toBeCloseTo(expected,10)
  }
  expect(built.report.mesh!.triangleCount).toBeGreaterThan(0)
})
it('preserves both radii and the angular gap of a retaining ring', async () => {
  const graph = compileProfile('retaining_ring','center:[0,0,0],inner_radius:3mm,outer_radius:5mm,gap_degrees:60deg')
  const built = await buildOwnNurbsAsync(graph,{action:'build'})
  const curve = built.report.definitions[graph.nodes[0]!.id] as NurbsCurve
  expect(Math.hypot(...curve.controlPoints[0]!)).toBeCloseTo(5,12)
  expect(curve.controlPoints.some(p => Math.abs(Math.hypot(...p)-3)<1e-10)).toBe(true)
  expect(curve.controlPoints[0]![0]).toBeCloseTo(5*Math.cos(Math.PI/6),10)
  expect(built.report.mesh!.triangleCount).toBeGreaterThan(0)
})
it('evaluates inner and outer radial boundaries of the annular sector', async () => {
  const {document} = compileRushFrontend('// @rush/1\ns = annular_sector_surface(center:[0,0,0],normal:[0,0,1],inner_radius:2mm,outer_radius:6mm,start_degrees:0deg,sweep_degrees:90deg)\nshow s.tessellate(8,2)')
  const node = document.nodes[0]!.id
  const r = await buildOwnNurbsAsync(document,{action:'evaluate',evaluations:[{node,u:0,v:0},{node,u:1,v:1}]})
  expect(Math.hypot(...r.evaluations[0]!.point)).toBeCloseTo(2,10)
  expect(Math.hypot(...r.evaluations[1]!.point)).toBeCloseTo(6,10)
  expect((await buildOwnNurbsAsync(document,{action:'build'})).report.mesh!.triangleCount).toBeGreaterThan(0)
})
it('rejects an inverted flange, fractional tooth count and an angle supplied for a width', async () => {
  await expect(buildOwnNurbsAsync(compileProfile('t_slot',cases[3]![1].replace('8mm','1mm')),{action:'build'})).rejects.toThrow()
  await expect(buildOwnNurbsAsync(compileProfile('toothed_belt',cases[6]![1].replace('teeth:3','teeth:1.5')),{action:'build'})).rejects.toThrow()
  expect(() => compileProfile('capsule',cases[1]![1].replace('4mm','4deg'))).toThrow()
})
