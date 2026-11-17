import { it, expect, beforeAll } from 'vitest'
import { bezierNurbsCurve, circleNurbsArc, circleNurbsCurve } from '../src/kernel/nurbsConstructors'
import { applySolidSweepBody } from '../src/kernel/solidSweepBody'
import { minimumPathCurvatureRadius, sampleSweepPath, sweepPathFrames } from '../src/kernel/solidSweepSmoothPath'
import { DirectInputError } from '../src/i18n/kernelErrors'
import { inspectNurbsBrep } from '../src/kernel/geometry/brep'
import * as geometryKernel from '../src/kernel/geometry/kernel'
import type { DirectDocument } from '../src/app/directModeling'
import type { NurbsCurve } from '../src/kernel/nurbsCurve'

type Vec3 = [number, number, number]
beforeAll(async () => {
  await geometryKernel.warmGeometryKernel()
})
const doc = (profile: NurbsCurve, path: NurbsCurve): DirectDocument =>
  ({
    version: 1,
    sketches: [],
    bodies: [],
    curves: [
      { id: 'prof', name: 'Profile', curve: profile },
      { id: 'path', name: 'Path', curve: path },
    ],
  }) as DirectDocument
const sweep = (profile: NurbsCurve, path: NurbsCurve, pathTolerance?: number) =>
  applySolidSweepBody(doc(profile, path), {
    profileId: 'prof',
    pathId: 'path',
    operation: 'new',
    targetId: '',
    id: 'a',
    name: 'A',
    pathTolerance,
  }).bodies[0]!
const volume = (mesh: { positions: ArrayLike<number>; indices: ArrayLike<number> }) => {
  let v = 0
  const p = mesh.positions,
    ix = mesh.indices
  for (let i = 0; i < ix.length; i += 3) {
    const a = ix[i]! * 3,
      b = ix[i + 1]! * 3,
      c = ix[i + 2]! * 3
    v +=
      p[a]! * (p[b + 1]! * p[c + 2]! - p[b + 2]! * p[c + 1]!) -
      p[a + 1]! * (p[b]! * p[c + 2]! - p[b + 2]! * p[c]!) +
      p[a + 2]! * (p[b]! * p[c + 1]! - p[b + 1]! * p[c]!)
  }
  return Math.abs(v / 6)
}
const code = (fn: () => unknown) => {
  try {
    fn()
  } catch (error) {
    return error instanceof DirectInputError ? `${error.code}: ${error.message}` : String(error)
  }
  return 'no error'
}
const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
const dist = (a: Vec3, b: Vec3) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2])

it('C9 sweep smooth: quarter arc path gives a capped tube with the exact bent-cylinder volume', () => {
  // Arc of radius 20 in XY from (20,0,0) to (0,20,0); profile radius 2 in the XZ plane at the start.
  const body = sweep(circleNurbsCurve([20, 0, 0], [0, 1, 0], 2), circleNurbsArc([0, 0, 0], [0, 0, 1], 20, 0, 90))
  expect(inspectNurbsBrep(body.brep!)).toMatchObject({ topologyValid: true, boundaryEdgeCount: 0 })
  const exact = Math.PI * 4 * ((Math.PI / 2) * 20)
  expect(Math.abs(volume(body.mesh) - exact) / exact).toBeLessThan(0.02)
  // The end cap sits at (0,20,0) normal to the arc tangent (-X), with the profile radius.
  const p = body.mesh.positions,
    cap: number[] = []
  for (let i = 0; i < p.length; i += 3)
    if (Math.abs(p[i]!) < 1e-6) cap.push(dist([p[i]!, p[i + 1]!, p[i + 2]!], [0, 20, 0]))
  expect(cap.length).toBeGreaterThan(1)
  expect(Math.max(...cap)).toBeCloseTo(2, 3)
})

it('C9 sweep smooth: a full circle path closes into a ring with the torus volume', () => {
  const body = sweep(circleNurbsCurve([20, 0, 0], [0, 1, 0], 2), circleNurbsCurve([0, 0, 0], [0, 0, 1], 20))
  expect(inspectNurbsBrep(body.brep!)).toMatchObject({ topologyValid: true, boundaryEdgeCount: 0 })
  const exact = 2 * Math.PI ** 2 * 20 * 4
  expect(Math.abs(volume(body.mesh) - exact) / exact).toBeLessThan(0.02)
})

it('C9 sweep smooth: a spline path keeps the profile normal to the curve at both ends', () => {
  const path = bezierNurbsCurve([
    [0, 0, 0],
    [0, 0, 10],
    [10, 5, 10],
    [10, 10, 20],
  ])
  const body = sweep(circleNurbsCurve([0, 0, 0], [0, 0, 1], 1), path)
  expect(inspectNurbsBrep(body.brep!)).toMatchObject({ topologyValid: true, boundaryEdgeCount: 0 })
  const { startTangent, endTangent } = sampleSweepPath(path, 1, ['path'])
  const exactEnd: Vec3 = [0, 5 / Math.hypot(5, 10), 10 / Math.hypot(5, 10)]
  expect(dot(startTangent, [0, 0, 1])).toBeCloseTo(1, 8)
  expect(dot(endTangent, exactEnd)).toBeCloseTo(1, 8)
  const p = body.mesh.positions,
    cap: number[] = []
  for (let i = 0; i < p.length; i += 3) {
    const q: Vec3 = [p[i]!, p[i + 1]!, p[i + 2]!]
    const d: Vec3 = [q[0] - 10, q[1] - 10, q[2] - 20]
    if (Math.abs(dot(d, exactEnd)) < 1e-6) cap.push(dist(q, [10, 10, 20]))
  }
  expect(cap.length).toBeGreaterThan(1)
  expect(Math.max(...cap)).toBeCloseTo(1, 3)
})

it('C9 sweep smooth: arc sampling meets the chordal tolerance and refines when it shrinks', () => {
  const arc = circleNurbsArc([0, 0, 0], [0, 0, 1], 20, 0, 90)
  for (const tolerance of [0.1, 0.01, 0.001]) {
    const { points } = sampleSweepPath(arc, 2, ['path'], tolerance)
    for (let i = 1; i < points.length; i++) {
      expect(Math.hypot(...points[i]!)).toBeCloseTo(20, 9)
      const half = dist(points[i]!, points[i - 1]!) / 2
      expect(20 - Math.sqrt(400 - half * half)).toBeLessThanOrEqual(tolerance * 1.0001)
    }
  }
  const coarse = sampleSweepPath(arc, 2, ['path'], 0.1).points.length,
    fine = sampleSweepPath(arc, 2, ['path'], 0.001).points.length
  expect(fine).toBeGreaterThan(coarse * 3)
  expect(minimumPathCurvatureRadius(sampleSweepPath(arc, 2, ['path']).points)).toBeCloseTo(20, 2)
})

it('C9 sweep smooth: helix frames are rotation-minimizing and never flip', () => {
  const points: Vec3[] = Array.from({ length: 401 }, (_, i) => {
    const a = (i / 400) * 4 * Math.PI
    return [10 * Math.cos(a), 10 * Math.sin(a), 3 * a]
  })
  const t0 = [0, 10, 3].map(v => v / Math.hypot(10, 3)) as Vec3
  const frames = sweepPathFrames(points, [1, 0, 0], false, t0)
  expect(Math.abs(dot(frames[0]!.r, t0))).toBeLessThan(1e-12)
  expect(frames[0]!.r[0]).toBeCloseTo(1, 12)
  for (let i = 1; i < frames.length; i++) {
    const a = frames[i - 1]!,
      b = frames[i]!
    expect(Math.abs(dot(b.r, b.t))).toBeLessThan(1e-9)
    expect(Math.abs(dot(b.r, b.r) - 1)).toBeLessThan(1e-12)
    // No rotation about the tangent: r does not move toward s between stations.
    expect(Math.abs(dot(b.r, a.s))).toBeLessThan(5e-4)
    expect(dot(a.r, b.r)).toBeGreaterThan(0.99)
  }
  // Analytic RMF of a helix turns against the Frenet normal at rate τ, so after two turns
  // (pitch angle α, cos α = 10/√109) r sits at -4π·sin α about the tangent from the normal.
  const end = frames.at(-1)!,
    normal: Vec3 = [-1, 0, 0],
    binormal: Vec3 = [
      end.t[1] * normal[2] - end.t[2] * normal[1],
      end.t[2] * normal[0] - end.t[0] * normal[2],
      end.t[0] * normal[1] - end.t[1] * normal[0],
    ]
  const sinA = 3 / Math.hypot(10, 3)
  const expected = -4 * Math.PI * sinA
  const angle = Math.atan2(dot(end.r, binormal), dot(end.r, normal))
  const wrap = (a: number) => Math.atan2(Math.sin(a), Math.cos(a))
  expect(Math.abs(wrap(angle - wrap(expected + Math.PI)))).toBeLessThan(1e-2)
})

it('C9 sweep smooth: refuses paths tighter than the profile radius with a registered code', () => {
  const message = code(() =>
    sweep(circleNurbsCurve([1.5, 0, 0], [0, 1, 0], 2), circleNurbsArc([0, 0, 0], [0, 0, 1], 1.5, 0, 90)),
  )
  expect(message).toMatch(/^DIRECT_SWEEP_PATH: The sweep path bends tighter/)
})

it('C9 sweep smooth: refuses self-intersecting paths and swept tubes that cut through themselves', () => {
  // Planar loop whose ends cross each other.
  const crossing = bezierNurbsCurve([
    [0, 0, 0],
    [0, 30, 0],
    [-15, -15, 0],
    [15, 15, 0],
  ])
  // A tiny profile, so only the crossing itself is in play.
  expect(code(() => sampleSweepPath(crossing, 1e-4, ['path']))).toBe(
    'DIRECT_SWEEP_PATH: The sweep path crosses itself.',
  )
  // The same loop lifted 1 mm at its far end: the path no longer crosses, but its stretches
  // pass 0.95 mm apart, so a 0.5 mm profile would cut through the swept tube.
  const nearMiss = bezierNurbsCurve([
    [0, 0, 0],
    [0, 30, 0],
    [-15, -15, 1],
    [15, 15, 1],
  ])
  expect(code(() => sampleSweepPath(nearMiss, 0.5, ['path']))).toMatch(
    /^DIRECT_SWEEP_PATH: The swept profile would cut through itself: path stretches come 0\.95\d+ mm apart/,
  )
  expect(code(() => sampleSweepPath(nearMiss, 0.4, ['path']))).toBe('no error')
})

it('C9 sweep smooth: refuses an off-plane profile and an invalid tolerance', () => {
  const arc = circleNurbsArc([0, 0, 0], [0, 0, 1], 20, 0, 90)
  expect(code(() => sweep(circleNurbsCurve([20, 0, 0], [0, 0, 1], 2), arc))).toMatch(
    /^DIRECT_SURFACE_INPUTS: The profile must lie in the plane normal/,
  )
  expect(code(() => sweep(circleNurbsCurve([20, 0, 0], [0, 1, 0], 2), arc, 0))).toMatch(/^DIRECT_SWEEP_PATH/)
})
