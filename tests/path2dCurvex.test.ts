import { describe, expect, it } from 'vitest'
import {
  addPathAnchors, concentricOffsetRings, gradientFillMesh, gradientStrokeMesh, gridArrayPaths,
  hatchRegion, knifeSplitPath, offsetPathRegion, pathDashSpans, pathFromPolygon, pathFromRect,
  pathToRing, puckerBloatPath, radialRepeatPaths, roughenPathDetailed, scatterPaths,
  splitPathAtAnchor, stepAndRepeat, stepAndRepeatPaths, stippleRegion, tessellatePathRegion,
  zigZagPathRidges,
  findGeometrySnap, roundedOpenPolylinePath, joinPathsAtTangents, snapToRays,
  hitTestCompoundPath, cutCompoundPathAtHit, knifeSplitCompoundPath,
  findDragSnap, computeDistanceMarks, intersectingPathDirections,
  type PathFillMesh, type PathGradient,
} from '../src/services/geometry/path2d'

function meshArea(mesh: PathFillMesh): number {
  let area = 0
  for (let i = 0; i < mesh.indices.length; i += 3) {
    const a = mesh.positions[mesh.indices[i]!]!, b = mesh.positions[mesh.indices[i + 1]!]!, c = mesh.positions[mesh.indices[i + 2]!]!
    area += Math.abs((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) / 2
  }
  return area
}

const gradient: PathGradient = {
  kind: 'linear', p1: [0, 0], p2: [10, 0],
  stops: [{ offset: 0, color: [255, 0, 0, 255] }, { offset: 1, color: [0, 0, 255, 255] }],
}

describe('Curvex 2D geometry through the actual WASM transport', () => {
  it('keeps compound hole topology when scissors open a ring or knife cuts only a hole', () => {
    const outer = pathFromRect([0, 0], [20, 20]), hole = pathFromRect([2, 2], [8, 8])
    const hit = hitTestCompoundPath([outer, hole], [2, 5], 0.1)!
    expect(hit.ring).toBe(1)
    const opened = cutCompoundPathAtHit([outer, hole], hit)
    expect(opened).toHaveLength(2)
    expect(opened[0]).toEqual([outer])
    expect(opened[1]![0]!.closed).toBe(false)
    const pieces = knifeSplitCompoundPath([outer, hole], [1, 5], [9, 5])
    expect(pieces).toHaveLength(1)
    expect(pieces[0]).toHaveLength(3)
    expect(meshArea(tessellatePathRegion({ rings: pieces[0]!.map(p => pathToRing(p)) }, { fillRule: 'evenodd' }))).toBeCloseTo(364, 9)
  })

  it('preserves semantic snap priorities, open corner styles and tangent joins', () => {
    expect(findGeometrySnap([4.8, 0], 0.3, [
      { type: 'line', start: [0, 0], end: [10, 0] },
      { type: 'line', start: [4.8, 0], end: [4.8, 10] },
    ])).toMatchObject({ point: [5, 0], kind: 'Midpoint' })
    expect(findGeometrySnap([2, 3], 0.1, [{ type: 'ellipse', center: [2, 3], radii: [10, 5] }])?.kind).toBe('Center')
    const rounded = roundedOpenPolylinePath([[0, 0], [10, 0], [10, 10]], [0, -2, 0])
    expect(rounded.segments.map(s => s.to)).toEqual([[8, 0], [10, 2], [10, 10]])
    expect(snapToRays([0, 0], [10, 0.1])?.point).toEqual([10, 0])
    expect(snapToRays([0, 0], [10, 2])).toBeNull()
    const joined = joinPathsAtTangents(pathFromPolygon([[0, 0], [2, 0]], false), pathFromPolygon([[4, 2], [4, 4]], false))
    expect(joined.segments).toHaveLength(4)
    expect(joined.segments[1]).toMatchObject({ type: 'cubic', to: [4, 0] })
    expect(findDragSnap({ min: [0, 0], max: [10, 10] }, [{ min: [1, 20], max: [11, 30] }], 1.1).delta).toEqual([1, 0])
    expect(computeDistanceMarks({ min: [0, 0], max: [10, 10] }, [{ min: [-3, -4], max: [15, 16] }]).map(m => m.distance)).toEqual([3, 5, 4, 6])
    expect(intersectingPathDirections([pathFromPolygon([[0, -2], [0, 2]], false)], [-1, 0], [1, 0])).toEqual([Math.PI / 2])
  })

  it('generates real ridges and bowed sides, with stable open endpoints', () => {
    const line = pathFromPolygon([[0, 0], [8, 0]], false)
    const zigzag = zigZagPathRidges(line, 1, 3)
    expect([zigzag.start, ...zigzag.segments.map(s => s.to)]).toEqual([[0, 0], [2, 1], [4, -1], [6, 1], [8, 0]])
    const rough = roughenPathDetailed(line, 0.5, 4, true)
    expect(rough.start).toEqual([0, 0])
    expect(rough.segments.at(-1)?.to).toEqual([8, 0])
    expect(rough.segments).toHaveLength(4)
    expect(rough.segments.every(s => s.type === 'cubic')).toBe(true)
    expect(rough).toEqual(roughenPathDetailed(line, 0.5, 4, true))
    const bloat = puckerBloatPath(pathFromRect([0, 0], [4, 2]), 0.5)
    expect(bloat.segments[0]).toEqual({ type: 'cubic', c1: [-1.75, -0.875], c2: [5.75, -0.875], to: [5, -0.5] })
  })

  it('provides copy-only grouped arrays while preserving old array defaults', () => {
    const source = pathFromRect([2, -0.5], [4, 0.5])
    expect(stepAndRepeat(source, 2, [10, 0])[0]?.start).toEqual(source.start)
    expect(stepAndRepeatPaths([source], 2, [10, 0])[0]?.start).toEqual([12, -0.5])
    expect(gridArrayPaths([source], 2, 2, [10, 20])).toHaveLength(3)
    const upright = radialRepeatPaths([source], 1, [0, 0], Math.PI / 2, false)[0]!
    expect(upright.start[0]).toBeCloseTo(-1, 10)
    expect(upright.start[1]).toBeCloseTo(2.5, 10)
    const second = pathFromRect([7, -0.5], [9, 0.5])
    const options = { position: 3, rotationRadians: Math.PI, scaleMin: 2, scaleMax: 2, seed: 13 }
    const copies = scatterPaths([source, second], 2, options)
    expect(copies).toEqual(scatterPaths([source, second], 2, options))
    expect(Math.hypot(copies[1]!.start[0] - copies[0]!.start[0], copies[1]!.start[1] - copies[0]!.start[1])).toBeCloseTo(10, 10)
  })

  it('keeps holes through fill tessellation, offsets, pattern fill and gradient export', () => {
    const outer = pathFromRect([0, 0], [10, 10]), hole = pathFromRect([4, 4], [6, 6])
    const region = { path: outer, holes: [hole] }
    expect(meshArea(tessellatePathRegion(region, { fillRule: 'evenodd' }))).toBeCloseTo(96, 9)
    expect(meshArea(tessellatePathRegion(region, { fillRule: 'nonzero' }))).toBeCloseTo(100, 9)
    expect(meshArea(gradientFillMesh(region, gradient, { fillRule: 'evenodd' }))).toBeCloseTo(96, 9)
    const offset = offsetPathRegion(region, 0.5, { fillRule: 'evenodd', miterLimit: 2, tolerance: 0.01 })
    expect(meshArea(tessellatePathRegion({ rings: offset.map(p => pathToRing(p)) }))).toBeCloseTo(120, 8)
    const nested = concentricOffsetRings([pathToRing(outer), pathToRing(hole)], 2, 0.2, 'Miter', 8, { fillRule: 'evenodd' })
    expect(nested).toHaveLength(2)
    expect(nested.every(rings => rings.length === 2)).toBe(true)
    const lines = hatchRegion(region, 1, { fillRule: 'evenodd' })
    expect(lines.length).toBeGreaterThan(0)
    for (const line of lines) {
      const end = line.segments.at(-1)!.to
      const middle = [(line.start[0] + end[0]) / 2, (line.start[1] + end[1]) / 2]
      expect(middle[0]! > 4 && middle[0]! < 6 && middle[1]! > 4 && middle[1]! < 6).toBe(false)
    }
    const dots = stippleRegion(region, 2, 0.2, { fillRule: 'evenodd', jitter: 0.1, seed: 42 })
    expect(dots.length).toBeGreaterThan(0)
    expect(dots).toEqual(stippleRegion(region, 2, 0.2, { fillRule: 'evenodd', jitter: 0.1, seed: 42 }))
  })

  it('exposes dash, split, node subdivision, and gradient stroke operations', () => {
    const line = pathFromPolygon([[0, 0], [10, 0]], false)
    expect(pathDashSpans(line, [2, 2])).toEqual([[[0, 0], [2, 0]], [[4, 0], [6, 0]], [[8, 0], [10, 0]]])
    const split = splitPathAtAnchor(addPathAnchors(line), 1)
    expect(split).toHaveLength(2)
    expect(split[0]!.segments.at(-1)!.to).toEqual(split[1]!.start)
    expect(knifeSplitPath(pathFromRect([0, 0], [10, 10]), [5, -1], [5, 11])).toHaveLength(2)
    const mesh = gradientStrokeMesh(line, 1, gradient, { cap: 'square' })
    expect(meshArea(mesh)).toBeCloseTo(11, 9)
    expect(mesh.colors).toHaveLength(mesh.positions.length)
  })
})
