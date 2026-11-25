import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

const source = readFileSync('examples/rush/original-intersections.r','utf8')
const build = async (code: string) => {
  const scene = await buildTextNurbsScene(compileRushFrontend(code).document)
  expect(scene.meshes).toHaveLength(1)
  return new Map(scene.measurements!.map(m=>[m.operation,JSON.parse(m.report)]))
}
const encloses = (v:number[], x:number) => v[0]! <= x && x <= v[1]!

it('locates original curve and chart intersections and the forward ray hit', async () => {
  const reports = await build(source)
  expect(reports.size).toBe(4)
  const plane = reports.get('curve_plane_roots')
  expect(plane.coverageResolved).toBe(true)
  expect(plane.cells.some((c:{rootBounds:number[]|null})=>c.rootBounds && encloses(c.rootBounds,0.5))).toBe(true)
  for(const name of ['curve_pair_crossings','curve_plane_chart_crossings']) {
    const r=reports.get(name)
    expect(r.complete).toBe(true)
    expect(r.unresolved).toHaveLength(0)
    expect(r.crossings).toHaveLength(1)
    for(const axis of r.crossings[0].positionBounds) expect(encloses(axis,0)).toBe(true)
  }
  const ray=reports.get('surface_ray_hits')
  expect(ray.complete).toBe(true)
  expect(ray.roots).toHaveLength(1)
  expect(encloses(ray.roots[0].rayParameter,3)).toBe(true)
})

it('separates a coincident domain from an isolated root and excludes backward ray hits', async () => {
  const reports=await build(source.replace('plane_normal:[1,0,0]','plane_normal:[0,0,1]').replace('ray_direction:[0,0,-1]','ray_direction:[0,0,1]'))
  expect(reports.get('curve_plane_roots').cells.some((c:{classification:string})=>c.classification==='Coincident')).toBe(true)
  expect(reports.get('surface_ray_hits')).toMatchObject({complete:true,roots:[]})
})
