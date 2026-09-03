import { describe, expect, it } from 'vitest'
import type { MeshData } from '../src/core/mesh'
import { OpenSCADParseError, parseOpenSCAD } from '../src/services/openscadParser'

const stableProfile = { languageProfile: 'openscad/stable-2021.01' as const }

function meshBounds(meshes: readonly MeshData[]) {
  const min = [Infinity, Infinity, Infinity]
  const max = [-Infinity, -Infinity, -Infinity]
  for (const mesh of meshes) {
    for (let vertex = 0; vertex < mesh.vertices.length; vertex += 6) {
      for (let axis = 0; axis < 3; axis++) {
        min[axis] = Math.min(min[axis], mesh.vertices[vertex + axis])
        max[axis] = Math.max(max[axis], mesh.vertices[vertex + axis])
      }
    }
  }
  return { min, max, size: max.map((value, axis) => value - min[axis]) }
}

describe('direct evaluator stable module slice', () => {
  it('resizes 3D children from their aggregate bounds and honors auto axes', async () => {
    const result = await parseOpenSCAD(`
      resize(newsize = [10, 0, 0], auto = [true, true, false])
        cube([5, 4, 1]);
    `, stableProfile)

    expect(result.meshes).toHaveLength(1)
    expect(meshBounds(result.meshes).size).toEqual([10, 8, 1])
    expect(result.volume).toBeCloseTo(80, 6)
  })

  it('uses the largest requested resize factor for auto axes, including shrink', async () => {
    const result = await parseOpenSCAD(
      'resize([2, 0, 0], auto = true) cube([4, 8, 12]);',
      stableProfile,
    )

    expect(meshBounds(result.meshes).size).toEqual([2, 4, 6])
    expect(result.volume).toBeCloseTo(48, 6)
  })

  it('resizes 2D geometry before extrusion', async () => {
    const result = await parseOpenSCAD(
      'linear_extrude(height = 2) resize([4, 6]) square([2, 3]);',
      stableProfile,
    )

    expect(meshBounds(result.meshes).size).toEqual([4, 6, 2])
    expect(result.volume).toBeCloseTo(48, 6)
  })

  it('rejects mixed-dimensional resize children with a positioned error', async () => {
    const failure = parseOpenSCAD(`
      resize([2, 2, 2]) {
        cube(1);
        square(1);
      }
    `, stableProfile)

    await expect(failure).rejects.toBeInstanceOf(OpenSCADParseError)
    await expect(failure).rejects.toThrow('resize() cannot mix 2D and 3D children')
  })

  it('computes a real 3D Minkowski sum', async () => {
    const result = await parseOpenSCAD(`
      minkowski() {
        cube(1);
        cube(1);
      }
    `, stableProfile)

    expect(result.meshes).toHaveLength(1)
    expect(meshBounds(result.meshes)).toMatchObject({
      min: [0, 0, 0],
      max: [2, 2, 2],
      size: [2, 2, 2],
    })
    expect(result.volume).toBeCloseTo(8, 6)
  })

  it('computes 2D Minkowski inputs and rejects mixed dimensions explicitly', async () => {
    const planar = await parseOpenSCAD(`
      linear_extrude(1)
        minkowski() {
          square(1);
          translate([2, 3]) square([3, 4]);
        }
    `, stableProfile)

    expect(meshBounds(planar.meshes)).toMatchObject({
      min: [2, 3, 0], max: [6, 8, 1], size: [4, 5, 1],
    })
    expect(planar.volume).toBeCloseTo(20, 6)

    await expect(parseOpenSCAD(
      'minkowski() { cube(1); square(1); }',
      stableProfile,
    )).rejects.toThrow('minkowski() cannot mix 2D and 3D children')
  })

  it('intersects every 3D intersection_for iteration', async () => {
    const result = await parseOpenSCAD(
      'intersection_for(i = [0:1]) translate([i * 0.5, 0, 0]) cube(1);',
      stableProfile,
    )

    expect(result.meshes).toHaveLength(1)
    expect(meshBounds(result.meshes)).toMatchObject({
      min: [0.5, 0, 0],
      max: [1, 1, 1],
      size: [0.5, 1, 1],
    })
    expect(result.volume).toBeCloseTo(0.5, 6)
  })

  it('intersects 2D intersection_for iterations before extrusion', async () => {
    const result = await parseOpenSCAD(`
      linear_extrude(height = 2)
        intersection_for(i = [0:1])
          translate([i * 0.5, 0]) square(1);
    `, stableProfile)

    expect(meshBounds(result.meshes).size).toEqual([0.5, 1, 2])
    expect(result.volume).toBeCloseTo(1, 6)
  })

  it('accepts a scalar intersection_for iterator and reports dimension errors', async () => {
    const scalar = await parseOpenSCAD(
      'intersection_for(i = 1) cube(1);',
      stableProfile,
    )
    expect(scalar.meshes).toHaveLength(1)
    expect(scalar.volume).toBeCloseTo(1, 6)

    const mixed = await parseOpenSCAD(
      'intersection_for(i = [0:1]) if (i) cube(1); else square(1);',
      stableProfile,
    )
    expect(mixed.meshes).toEqual([])
    expect(mixed.warnings).toContain(
      'intersection_for() ignored child geometry with a different dimension',
    )
  })

  it('emits OpenSCAD-formatted echo effects without changing child geometry', async () => {
    const result = await parseOpenSCAD(
      'echo("size", n = 2, values = [1, true, undef]) cube(1);',
      stableProfile,
    )

    expect(result.warnings).toEqual(['ECHO: "size", n = 2, values = [1, true, undef]'])
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(1, 6)
  })

  it('supports a childless, argumentless echo effect', async () => {
    const result = await parseOpenSCAD('echo();', stableProfile)

    expect(result.warnings).toEqual(['ECHO:'])
    expect(result.meshes).toHaveLength(0)
  })
})
