import { describe, expect, it } from 'vitest'
import type { GeometryEvaluationResult } from '../src/core/build'
import { OpenSCADParseError, parseOpenSCAD } from '../src/services/openscadParser'

const stable = { languageProfile: 'openscad/stable-2021.01' as const }

type Vec3 = [number, number, number]

function bounds(result: GeometryEvaluationResult) {
  const min = [Infinity, Infinity, Infinity]
  const max = [-Infinity, -Infinity, -Infinity]
  for (const mesh of result.meshes) {
    for (let offset = 0; offset < mesh.vertices.length; offset += 6) {
      for (let axis = 0; axis < 3; axis++) {
        min[axis] = Math.min(min[axis], mesh.vertices[offset + axis])
        max[axis] = Math.max(max[axis], mesh.vertices[offset + axis])
      }
    }
  }
  const canonicalZero = (value: number) => Object.is(value, -0) ? 0 : value
  return {
    min: min.map(canonicalZero),
    max: max.map(canonicalZero),
    size: max.map((value, axis) => canonicalZero(value - min[axis])),
  }
}

function triangles(result: GeometryEvaluationResult): number {
  return result.meshes.reduce((count, mesh) => count + mesh.indices.length / 3, 0)
}

describe('direct stable geometry-module helper wiring', () => {
  it('uses the 2021 linear_extrude height default and makes non-positive height empty', async () => {
    const defaultHeight = await parseOpenSCAD('linear_extrude() square(1);', stable)
    const zero = await parseOpenSCAD('linear_extrude(height=0) square(1);', stable)
    const negative = await parseOpenSCAD('linear_extrude(height=-4) square(1);', stable)

    expect(bounds(defaultHeight)).toMatchObject({ size: [1, 1, 100] })
    expect(defaultHeight.volume).toBeCloseTo(100, 6)
    expect(zero.meshes).toEqual([])
    expect(negative.meshes).toEqual([])
  })

  it('adapts OpenSCAD slices to Manifold divisions and matches automatic twist slices', async () => {
    const one = await parseOpenSCAD(
      'linear_extrude(height=1,twist=90,slices=1) square(1);', stable,
    )
    const two = await parseOpenSCAD(
      'linear_extrude(height=1,twist=90,slices=2) square(1);', stable,
    )
    const automatic = await parseOpenSCAD(
      'linear_extrude(height=1,twist=90,$fn=40) square(1);', stable,
    )
    const explicit = await parseOpenSCAD(
      'linear_extrude(height=1,twist=90,slices=10,$fn=40) square(1);', stable,
    )

    expect(triangles(one)).toBe(12)
    expect(triangles(two)).toBe(20)
    expect(triangles(automatic)).toBe(84)
    expect(automatic.meshes[0].geometryAssetId).toBe(explicit.meshes[0].geometryAssetId)
  })

  it('binds every 2021 linear_extrude positional slot exactly like its named form', async () => {
    const positional = await parseOpenSCAD(
      'linear_extrude(2,true,10,90,4,[0.5,1]) square(1);', stable,
    )
    const named = await parseOpenSCAD(`
      linear_extrude(
        height=2, center=true, convexity=10, twist=90, slices=4, scale=[0.5,1]
      ) square(1);
    `, stable)
    const namedThenPositional = await parseOpenSCAD(
      'linear_extrude(height=2,true) square(1);', stable,
    )

    expect(positional.meshes[0].geometryAssetId).toBe(named.meshes[0].geometryAssetId)
    expect(bounds(positional).min[2]).toBe(-1)
    expect(bounds(positional).max[2]).toBe(1)
    expect(bounds(namedThenPositional).min[2]).toBe(-1)
    expect(bounds(namedThenPositional).max[2]).toBe(1)
  })

  it('evaluates each extrusion argument once and reports duplicate/unknown names softly', async () => {
    const result = await parseOpenSCAD(`
      linear_extrude(
        echo("positional-height") 1,
        height=echo("named-height") 2,
        mystery=echo("unknown") 7,
        $fn=echo("fn") 8
      ) square(1);
    `, stable)

    expect(bounds(result).size[2]).toBe(2)
    for (const label of ['positional-height', 'named-height', 'unknown', 'fn']) {
      expect(result.warnings.filter(warning => warning === `ECHO: "${label}"`)).toHaveLength(1)
    }
    expect(result.warnings).toEqual(expect.arrayContaining([
      'Argument height was specified more than once for linear_extrude()',
      'Ignoring unknown argument mystery to linear_extrude()',
    ]))
  })

  it('clamps negative scale and keeps an invalid one-element scale vector soft', async () => {
    const cone = await parseOpenSCAD(
      'linear_extrude(height=2,scale=-1) square(1);', stable,
    )
    const invalid = await parseOpenSCAD(
      'linear_extrude(height=2,scale=[2]) square(1);', stable,
    )

    expect(bounds(cone).size).toEqual([1, 1, 2])
    expect(bounds(invalid).size).toEqual([1, 1, 2])
    expect(invalid.volume).toBeCloseTo(2, 6)
    expect(invalid.warnings).toContain('Invalid linear_extrude scale was replaced with [1, 1]')
  })

  it('applies 2D multmatrix before extrusion, including homogeneous normalization', async () => {
    const transformed = await parseOpenSCAD(`
      linear_extrude(1)
        multmatrix([
          [4, 0, 0, 8],
          [0, 6, 0, 10],
          [0, 0, 2, 0],
          [0, 0, 0, 2]
        ]) square(1);
    `, stable)

    expect(bounds(transformed)).toMatchObject({
      min: [4, 5, 0], max: [6, 8, 1], size: [2, 3, 1],
    })
    expect(transformed.volume).toBeCloseTo(6, 6)
  })

  it('anchors 2D Minkowski at a contained contour point for translated curved profiles', async () => {
    const result = await parseOpenSCAD(`
      linear_extrude(1)
        minkowski() {
          square(1);
          translate([3, 4]) circle(1, $fn=32);
        }
    `, stable)

    expect(bounds(result)).toMatchObject({
      min: [2, 3, 0], max: [5, 6, 1], size: [3, 3, 1],
    })
    const regular32GonArea = 16 * Math.sin(2 * Math.PI / 32)
    expect(result.volume).toBeCloseTo(regular32GonArea + 5, 6)
  })

  it('normalizes every translated right operand in an ordered 3D Minkowski sum', async () => {
    const pair = await parseOpenSCAD(`
      minkowski() {
        cube(1);
        translate([2,3,4]) cube([3,4,5]);
      }
    `, stable)
    const three = await parseOpenSCAD(`
      minkowski() {
        translate([5,7,11]) cube(1);
        translate([2,3,4]) cube([3,4,5]);
        translate([-1,6,-2]) cube([2,1,3]);
      }
    `, stable)

    expect(bounds(pair)).toMatchObject({
      min: [2, 3, 4], max: [6, 8, 10], size: [4, 5, 6],
    })
    expect(pair.volume).toBeCloseTo(120, 6)
    expect(bounds(three)).toMatchObject({
      min: [6, 16, 13], max: [12, 22, 22], size: [6, 6, 9],
    })
    expect(three.volume).toBeCloseTo(324, 6)
  })

  it('executes scalar, vector, and descending-range children selection with soft skips', async () => {
    const vector = await parseOpenSCAD(`
      module pick(which) { children(which); }
      pick([2, 0, 2]) {
        translate([20, 0, 0]) cube(3);
        translate([10, 0, 0]) cube(2);
        cube(1);
      }
    `, stable)
    const range = await parseOpenSCAD(`
      module pick(which) { children(which); }
      pick([2:-1:0]) { cube(3); cube(2); cube(1); }
    `, stable)
    const soft = await parseOpenSCAD(`
      module pick(which) { children(which); }
      pick([undef, -1, 1, 9]) { cube(1); cube(2); cube(3); }
    `, stable)

    expect(vector.meshes).toHaveLength(1)
    expect(vector.volume).toBeCloseTo(28, 6)
    expect(range.meshes).toHaveLength(1)
    expect(range.volume).toBeCloseTo(27, 6)
    expect(soft.meshes).toHaveLength(1)
    expect(soft.volume).toBeCloseTo(8, 6)
    expect(soft.warnings).toEqual(expect.arrayContaining([
      'Non-numeric children index was ignored',
      'Children index -1 is outside 0..2',
      'Children index 9 is outside 0..2',
    ]))
  })

  it('uses aggregate resize bounds and the maximum authored factor for auto axes', async () => {
    const result = await parseOpenSCAD(
      'resize([100,40,0],auto=true) cube([10,20,30]);', stable,
    )

    expect(bounds(result).size).toEqual([100, 40, 300])
    expect(result.volume).toBeCloseTo(1_200_000, 4)
  })

  it('accepts and evaluates resize convexity as a compatibility-only third slot', async () => {
    const positional = await parseOpenSCAD(
      'resize([2,2,2],true,echo("resize-convexity") 10) cube(1);', stable,
    )
    const named = await parseOpenSCAD(
      'resize(newsize=[2,2,2],auto=true,convexity=10) cube(1);', stable,
    )

    expect(positional.meshes[0].geometryAssetId).toBe(named.meshes[0].geometryAssetId)
    expect(positional.warnings.filter(warning => warning === 'ECHO: "resize-convexity"'))
      .toHaveLength(1)
  })

  it('normalizes rotate_extrude angles and supports negative-X profiles', async () => {
    const positive = await parseOpenSCAD(
      'rotate_extrude(angle=90,$fn=8) translate([2,0]) square(1);', stable,
    )
    const negative = await parseOpenSCAD(
      'rotate_extrude(angle=90,$fn=8) translate([-3,0]) square(1);', stable,
    )
    const fullNegative = await parseOpenSCAD(
      'rotate_extrude(angle=-360,$fn=8) translate([2,0]) square(1);', stable,
    )
    const overFull = await parseOpenSCAD(
      'rotate_extrude(angle=450,$fn=8) translate([2,0]) square(1);', stable,
    )
    const zero = await parseOpenSCAD(
      'rotate_extrude(angle=0,$fn=8) translate([2,0]) square(1);', stable,
    )

    // Bounds come out of f32 kernel arithmetic, so exact zeros can surface as
    // ~1e-16 residues (rotate_extrude at 90°). Compare component-wise with a
    // 1e-6 tolerance: far above float noise, tight enough for real regressions.
    const expectBoundsClose = (
      result: GeometryEvaluationResult,
      expected: { min: Vec3; max: Vec3 },
    ) => {
      const actual = bounds(result)
      for (const axis of [0, 1, 2] as const) {
        expect(Math.abs(actual.min[axis]! - expected.min[axis]!)).toBeLessThanOrEqual(1e-6)
        expect(Math.abs(actual.max[axis]! - expected.max[axis]!)).toBeLessThanOrEqual(1e-6)
      }
    }
    expectBoundsClose(positive, { min: [0, 0, 0], max: [3, 3, 1] })
    expectBoundsClose(negative, { min: [-3, -3, 0], max: [0, 0, 1] })
    expect(fullNegative.meshes[0].geometryAssetId).toBe(overFull.meshes[0].geometryAssetId)
    expect(zero.meshes).toEqual([])

    const crossing = parseOpenSCAD(
      'rotate_extrude(angle=90) translate([-0.5,0]) square(1);', stable,
    )
    await expect(crossing).rejects.toBeInstanceOf(OpenSCADParseError)
    await expect(crossing).rejects.toThrow('profile may not cross the Y axis')
  })

  it('binds rotate_extrude angle/convexity positionally and evaluates the hint once', async () => {
    const positional = await parseOpenSCAD(
      'rotate_extrude(90,echo("rotate-convexity") 10,$fn=8) translate([2,0]) square(1);',
      stable,
    )
    const named = await parseOpenSCAD(
      'rotate_extrude(angle=90,convexity=10,$fn=8) translate([2,0]) square(1);',
      stable,
    )

    expect(positional.meshes[0].geometryAssetId).toBe(named.meshes[0].geometryAssetId)
    expect(positional.warnings.filter(warning => warning === 'ECHO: "rotate-convexity"'))
      .toHaveLength(1)
  })
})
