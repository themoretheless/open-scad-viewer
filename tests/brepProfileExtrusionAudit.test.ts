import { expect, it } from 'vitest'
import { createBrepSemanticBackend, inspectBrepSemanticPayload } from '../src/services/brepSemanticBackend'
import { executeSemanticProgram } from '../src/services/semanticProgramExecutor'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'
import {
  analyzeNurbsBrep,
  booleanNurbsBrep,
  createBrepCylinder,
  extrudeBrepCurves,
  type NurbsBrep,
} from '../src/services/geometry/brep'
import { validateBrepProfile } from '../src/services/geometry/brepProfile'
import { elevateNurbsCurve, insertNurbsKnot, type NurbsCurve } from '../src/services/nurbsCurve'

function circle(): NurbsCurve {
  return {
    degree: 2,
    knots: [0, 0, 0, .25, .25, .5, .5, .75, .75, 1, 1, 1],
    controlPoints: [[3, 0], [3, 3], [0, 3], [-3, 3], [-3, 0], [-3, -3], [0, -3], [3, -3], [3, 0]],
    weights: [1, Math.SQRT1_2, 1, Math.SQRT1_2, 1, Math.SQRT1_2, 1, Math.SQRT1_2, 1],
  }
}

const lower = (source: string) => lowerOpenSCADToSemanticProgram('// @language openscad-viewer/brep-1\n' + source)

async function solid(source: string): Promise<NurbsBrep> {
  const run = await executeSemanticProgram(lower(source), createBrepSemanticBackend())
  try {
    expect(run.outputs).toHaveLength(1)
    const output = run.outputs[0].value
    if (output.tag !== 'value') throw new Error('Expected a nonempty solid')
    return inspectBrepSemanticPayload(output.payload).model
  } finally {
    await run.dispose()
  }
}

function mass(model: NurbsBrep, volume: number, centroid: number[]) {
  const report = analyzeNurbsBrep(model)
  expect(report.signedVolumeMm3).toBeCloseTo(volume, 6)
  report.centroid.forEach((coordinate, i) => expect(coordinate).toBeCloseTo(centroid[i], 8))
}

it('keeps refined full-circle geometry invariant under knot-domain and weight basis changes', () => {
  let curve = circle()
  for (const knot of [.07, .31, .68, .93]) curve = insertNurbsKnot(curve, knot, 1)
  curve = { ...curve, knots: curve.knots.map(t => -7 + 16 * t), weights: curve.weights.map(w => 37 * w) }
  const source = JSON.stringify(curve)
  const profile = validateBrepProfile([[curve]])
  expect(profile.areaMm2).toBeCloseTo(9 * Math.PI, 10)
  expect(profile.loops[0][0]).toEqual(curve)
  const body = extrudeBrepCurves(profile.loops, 0, 5)
  expect(body.faces).toHaveLength(10)
  mass(body, 45 * Math.PI, [0, 0, 2.5])
  const removed = booleanNurbsBrep(JSON.parse(JSON.stringify(body)), createBrepCylinder(3, 5), 'difference')
  expect(removed.bodies).toEqual([])
  expect(JSON.stringify(curve)).toBe(source)
})

it('composes a transformed reflected profile extrusion with a native closed cylinder', async () => {
  const body = await solid(`difference() {
    linear_extrude(height=4) translate([7,-3]) rotate(137) mirror([1,2])
      difference() { circle(3); circle(1); }
    translate([7,-3,0]) cylinder(r=2,h=4);
  }`)
  expect(body.bodies).toHaveLength(1)
  expect(body.faces.filter(face => face.holes.length)).toHaveLength(2)
  mass(body, 20 * Math.PI, [7, -3, 2])
})

it('distinguishes admitted 3D affine solids from circular 2D profile admission', async () => {
  const body = await solid(`multmatrix([[-2,1,0,7],[0,3,0,-4],[0,0,.5,2],[0,0,0,1]])
    linear_extrude(height=4,center=true) circle(3);`)
  mass(body, 108 * Math.PI, [7, -4, 2])
  const polygon = await solid(`linear_extrude(height=4,center=true)
    translate([7,-4]) scale([2,3]) rotate(37) mirror([1,2]) square([2,1],center=true);`)
  mass(polygon, 48, [7, -4, 0])
  await expect(solid('linear_extrude(height=4) scale([2,3]) rotate(37) circle(3);'))
    .rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_FAILURE', backendCause: { code: 'E_BREP_SEMANTIC_UNSUPPORTED' } })
})

it('refuses implicit 3D projection and unsupported curve bases instead of dropping coordinates', async () => {
  const planar3d = { ...circle(), controlPoints: circle().controlPoints.map(p => [...p, 0]) }
  expect(() => validateBrepProfile([[planar3d]])).toThrow(/2D/)
  expect(() => extrudeBrepCurves([[planar3d]], 0, 2)).toThrow(/2D/)
  expect(() => validateBrepProfile([[elevateNurbsCurve(circle(), 3)]])).toThrow(/line|quadratic|circular/i)
  expect(() => lower('linear_extrude(height=2) multmatrix([[1,0,0,0],[0,1,0,0],[0,0,1,1],[0,0,0,1]]) square(2);'))
    .toThrow(/must preserve the XY plane/)
  await expect(solid('linear_extrude(height=2) projection() cube(1);'))
    .rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_FAILURE', backendCause: { code: 'E_BREP_SEMANTIC_UNSUPPORTED' } })
})

it('retains XY matrix profile transforms with independent area, volume and centroid checks', async () => {
  const polygon = await solid(`linear_extrude(height=4,center=true)
    multmatrix([[-2,1,0,7],[0,3,0,-4],[0,0,1,0],[0,0,0,1]]) square([2,1],center=true);`)
  mass(polygon, 48, [7, -4, 0])
  const ring = await solid(`linear_extrude(height=3,center=true)
    multmatrix([[0,2,0,7],[2,0,0,-4],[0,0,1,0],[0,0,0,1]])
      difference(){circle(3);circle(1);}`)
  mass(ring, 96 * Math.PI, [7, -4, 0])
  await expect(solid(`linear_extrude(height=2)
    multmatrix([[1,.1,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]) circle(3);`))
    .rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_FAILURE', backendCause: { code: 'E_BREP_SEMANTIC_UNSUPPORTED' } })
  expect(() => lowerOpenSCADToSemanticProgram('multmatrix([[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]) square(2);'))
    .toThrow(/multmatrix currently supports 3D children only/)
})
