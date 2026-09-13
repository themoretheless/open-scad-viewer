import {describe, expect, it} from 'vitest'
import {
  analyzeNurbsBrep, booleanNurbsBrep, createBrepBox, createBrepCylinder,
  inspectNurbsBrep, tessellateNurbsBrep, transformNurbsBrep,
  type BrepBooleanOperation,
} from '../src/services/geometry/brep'

const translated = (model: ReturnType<typeof createBrepCylinder>, x: number, z: number) =>
  transformNurbsBrep(model, [[1,0,0,x], [0,1,0,0], [0,0,1,z], [0,0,0,1]])
const lens = (a: number, b: number, d: number) => a*a*Math.acos((d*d+a*a-b*b)/(2*d*a))
  + b*b*Math.acos((d*d+b*b-a*a)/(2*d*b))
  - 0.5*Math.sqrt((-d+a+b)*(d+a-b)*(d-a+b)*(d+a+b))

describe('stepped analytic B-rep through the shipped WASM and shared-edge tessellator', () => {
  for (const operation of ['union', 'intersection', 'difference', 'xor'] as const) {
    it(`publishes a closed ${operation} result with the correct analytic volume`, () => {
      const a = createBrepCylinder(5, 8), b = translated(createBrepCylinder(3, 6), 3, 4)
      const before = JSON.stringify([a,b])
      const overlap = 4*lens(5,3,3), va = 200*Math.PI, vb = 54*Math.PI
      const expected: Record<BrepBooleanOperation,number> = {union: va+vb-overlap, difference: va-overlap, intersection: overlap, xor: va+vb-2*overlap}
      const result = booleanNurbsBrep(a,b,operation)
      expect(inspectNurbsBrep(result).topologyValid).toBe(true)
      expect(analyzeNurbsBrep(result).signedVolumeMm3).toBeCloseTo(expected[operation], 4)
      expect(JSON.stringify([a,b])).toBe(before)
      for (const detail of [1,4,8]) {
        const mesh = tessellateNurbsBrep(result,detail)
        expect(mesh.report).toMatchObject({closed: true, boundaryEdges: 0, nonManifoldEdges: 0, orientationConflicts: 0, degenerateTriangles: 0})
        expect(mesh.faceIds).toHaveLength(mesh.indices.length/3)
        expect(mesh.faceIds.every(face => face >= 0 && face < result.faces.length)).toBe(true)
        expect(mesh.report.triangleCount).toBeLessThanOrEqual(20_000)
      }
      const restored = JSON.parse(JSON.stringify(result))
      expect(inspectNurbsBrep(restored).topologyValid).toBe(true)
      expect(restored.topologyIds).toEqual(result.topologyIds)
    }, 20_000)
  }

  it('keeps the inner shell of a sealed cavity through display and native roundtrip', () => {
    const box = createBrepBox([-5,-5,0],[5,5,10])
    const cutter = translated(createBrepCylinder(2,6),0,2)
    const cut = booleanNurbsBrep(box,cutter,'difference')
    expect(cut.bodies).toHaveLength(1)
    expect(cut.bodies[0].innerShells).toHaveLength(1)
    expect(analyzeNurbsBrep(cut).signedVolumeMm3).toBeCloseTo(1000-24*Math.PI,4)
    expect(tessellateNurbsBrep(cut,8).report).toMatchObject({closed:true, boundaryEdges:0, nonManifoldEdges:0, orientationConflicts:0})
    const restored = JSON.parse(JSON.stringify(cut))
    const filled = booleanNurbsBrep(restored,cutter,'union')
    expect(analyzeNurbsBrep(filled).signedVolumeMm3).toBeCloseTo(1000,4)
    expect(filled.bodies[0].innerShells).toEqual([])
  }, 20_000)

  it('reuses a rotated stepped result after serialization', () => {
    const a = createBrepCylinder(5,8), b = translated(createBrepCylinder(3,6),3,4)
    const pose = [[.8,0,.6,10],[.36,.8,-.48,-4],[-.48,.6,.64,2],[0,0,0,1]]
    const union = booleanNurbsBrep(a,b,'union')
    const placed = JSON.parse(JSON.stringify(transformNurbsBrep(union,pose)))
    const cut = booleanNurbsBrep(placed,transformNurbsBrep(b,pose),'difference')
    expect(analyzeNurbsBrep(cut).signedVolumeMm3).toBeCloseTo(200*Math.PI-4*lens(5,3,3),4)
    expect(tessellateNurbsBrep(cut,4).report).toMatchObject({closed:true, boundaryEdges:0, nonManifoldEdges:0})
  }, 30_000)
})
