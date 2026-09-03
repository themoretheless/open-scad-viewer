import { describe, expect, it } from 'vitest'
import type { GeometryEvaluationResult } from '../src/core/build'
import { OpenSCADParseError, parseOpenSCAD } from '../src/services/openscadParser'

const stable = { languageProfile: 'openscad/stable-2021.01' as const }

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
  return { min, max, size: max.map((value, axis) => value - min[axis]) }
}

describe('direct OpenSCAD 2021 primitive and transform plan wiring', () => {
  it('evaluates primitive arguments once before suppressing primitive children', async () => {
    const result = await parseOpenSCAD(`
      cube(
        size=echo("primitive-arg") [2,"bad",4],
        mystery=echo("ignored-arg") 12
      ) {
        echo("primitive-child") cube(99);
      }
    `, stable)

    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(2, 6)
    expect(bounds(result).size).toEqual([2, 1, 1])
    expect(result.warnings.filter(warning => warning === 'ECHO: "primitive-arg"')).toHaveLength(1)
    expect(result.warnings.filter(warning => warning === 'ECHO: "ignored-arg"')).toHaveLength(1)
    expect(result.warnings).not.toContain('ECHO: "primitive-child"')
    expect(result.warnings).toEqual(expect.arrayContaining([
      'Ignoring unknown argument mystery to cube()',
      expect.stringContaining('cube size was not a scalar'),
      'cube() ignores child geometry',
    ]))
  })

  it('keeps radius/diameter priority and composes common and end cylinder radii', async () => {
    const diameter = await parseOpenSCAD('sphere(r=9,d=2,$fn=8);', stable)
    const diameterOnly = await parseOpenSCAD('sphere(d=2,$fn=8);', stable)
    const circle = await parseOpenSCAD('linear_extrude(1) circle(r=9,d=2,$fn=8);', stable)
    const circleDiameterOnly = await parseOpenSCAD('linear_extrude(1) circle(d=2,$fn=8);', stable)
    const mixedCylinder = await parseOpenSCAD('cylinder(h=2,r=2,r2=1,$fn=4);', stable)
    const explicitCylinder = await parseOpenSCAD('cylinder(h=2,r1=2,r2=1,$fn=4);', stable)

    expect(diameter.meshes[0].geometryAssetId).toBe(diameterOnly.meshes[0].geometryAssetId)
    expect(diameter.warnings).toContain('sphere uses d; the paired r value has no effect')
    expect(circle.meshes[0].geometryAssetId).toBe(circleDiameterOnly.meshes[0].geometryAssetId)
    expect(circle.warnings).toContain('circle uses d; the paired r value has no effect')
    expect(mixedCylinder.meshes[0].geometryAssetId).toBe(explicitCylinder.meshes[0].geometryAssetId)
    expect(mixedCylinder.volume).toBeCloseTo(28 / 3, 6)
    expect(mixedCylinder.warnings).toContain('cylinder combines a shared radius with an end-specific radius')
  })

  it('turns invalid ranges into empty geometry without throwing', async () => {
    for (const source of [
      'cube(0);',
      'sphere(r=-1);',
      'cylinder(h=0,r=1);',
      'linear_extrude(1) square([1,0]);',
    ]) {
      const result = await parseOpenSCAD(source, stable)
      expect(result.meshes, source).toEqual([])
      expect(result.warnings.some(warning => warning.includes('empty object')), source).toBe(true)
    }

    const malformedSquare = await parseOpenSCAD(
      'linear_extrude(1) square([2,3,4]);',
      stable,
    )
    expect(malformedSquare.volume).toBeCloseTo(1, 6)
    expect(malformedSquare.warnings).toContain(
      'square size was not a scalar or exact 2-component numeric vector; unit size is used',
    )
  })

  it('accepts vec2 polyhedron points and skips out-of-range polygon path entries', async () => {
    const polyhedron = await parseOpenSCAD(`
      polyhedron(
        points=[[0,0],[1,0],[0,1],[0,0,1]],
        faces=[[0,1,2],[0,3,1],[1,3,2],[2,3,0]]
      );
    `, stable)
    const polygon = await parseOpenSCAD(`
      linear_extrude(1)
        polygon(points=[[0,0],[1,0],[0,1]],paths=[[0,99,1,2]]);
    `, stable)

    expect(polyhedron.meshes).toHaveLength(1)
    expect(polyhedron.volume).toBeCloseTo(1 / 6, 6)
    expect(polygon.meshes).toHaveLength(1)
    expect(polygon.volume).toBeCloseTo(0.5, 6)
    expect(polygon.warnings).toContain(
      'polygon skipped a path entry whose point index is outside the point vector',
    )
  })

  it('evaluates transform arguments before children and unions children before transforming', async () => {
    const ordered = await parseOpenSCAD(`
      translate(echo("transform-arg") [0,0,0]) {
        echo("transform-child") cube(1);
      }
    `, stable)
    const unioned = await parseOpenSCAD(`
      intersection() {
        scale(1) {
          cube(1);
          translate([2,0,0]) cube(1);
        }
        translate([-1,-1,-1]) cube([5,3,3]);
      }
    `, stable)

    const argumentIndex = ordered.warnings.indexOf('ECHO: "transform-arg"')
    const childIndex = ordered.warnings.indexOf('ECHO: "transform-child"')
    expect(argumentIndex).toBeGreaterThanOrEqual(0)
    expect(childIndex).toBeGreaterThan(argumentIndex)
    expect(unioned.meshes).toHaveLength(1)
    expect(unioned.volume).toBeCloseTo(2, 6)
  })

  it('uses normalized matrices for 2D/3D and makes singular transforms empty', async () => {
    const partialMatrix = await parseOpenSCAD('multmatrix([[2]]) cube(1);', stable)
    const translated = await parseOpenSCAD('translate([2,3,4]) cube(1);', stable)
    const mirrored = await parseOpenSCAD('mirror([1,0,0]) cube([1,2,3]);', stable)
    const normalized2d = await parseOpenSCAD(`
      linear_extrude(1)
        multmatrix([
          [4,0,0,8], [0,6,0,10], [0,0,2,0], [0,0,0,2]
        ]) square(1);
    `, stable)
    const singular3d = await parseOpenSCAD('scale([0,1,1]) cube(1);', stable)
    const nonfinite = await parseOpenSCAD('scale([1e309,1,1]) cube(1);', stable)
    const singular2d = await parseOpenSCAD(
      'linear_extrude(1) rotate(a=90,v=[1,0,0]) square(1);',
      stable,
    )

    expect(partialMatrix.volume).toBeCloseTo(2, 6)
    expect(bounds(translated)).toMatchObject({ min: [2, 3, 4], max: [3, 4, 5] })
    expect(bounds(mirrored)).toMatchObject({ min: [-1, 0, 0], max: [0, 2, 3] })
    expect(bounds(normalized2d)).toMatchObject({ min: [4, 5, 0], max: [6, 8, 1] })
    expect(normalized2d.volume).toBeCloseTo(6, 6)
    expect(singular3d.meshes).toEqual([])
    expect(nonfinite.meshes).toEqual([])
    expect(nonfinite.warnings).toContain(
      'scale produced a non-finite matrix, so its child geometry is removed',
    )
    expect(singular2d.meshes).toEqual([])
  })

  it('binds a positional rotate axis after named a and keeps legacy behavior frozen', async () => {
    const mixed = await parseOpenSCAD('rotate(a=90,[1,0,0]) cube([1,2,3]);', stable)
    const named = await parseOpenSCAD('rotate(a=90,v=[1,0,0]) cube([1,2,3]);', stable)
    expect(mixed.meshes[0].geometryAssetId).toBe(named.meshes[0].geometryAssetId)

    await expect(parseOpenSCAD('cube([2,"bad",4]);')).rejects.toBeInstanceOf(OpenSCADParseError)
    const legacy = await parseOpenSCAD(`
      scale([1,1,1]) { cube(2); translate([1,0,0]) cube(2); }
    `)
    expect(legacy.meshes).toHaveLength(2)
    expect(legacy.volume).toBeCloseTo(16, 6)
  })
})
