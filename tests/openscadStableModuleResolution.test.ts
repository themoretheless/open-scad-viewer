import { describe, expect, it } from 'vitest'
import { parseOpenSCAD } from '../src/services/openscadParser'

const stable = { languageProfile: 'openscad/stable-2021.01' as const }

function triangleCount(result: Awaited<ReturnType<typeof parseOpenSCAD>>): number {
  return result.meshes.reduce((sum, mesh) => sum + mesh.indices.length / 3, 0)
}

describe('direct stable module resolution wiring', () => {
  it('uses the complete stable color resolver without changing legacy color rules', async () => {
    const named = await parseOpenSCAD('color("RebeccaPurple") cube(1);', stable)
    const shortHex = await parseOpenSCAD('color("#abcd") cube(1);', stable)
    const vector = await parseOpenSCAD('color([0.1, 0.2], 0.7) cube(1);', stable)

    expect(named.meshes[0].color).toEqual([0.4, 0.2, 0.6, 1])
    expect(shortHex.meshes[0].color).toEqual([0xaa / 255, 0xbb / 255, 0xcc / 255, 0xdd / 255])
    expect(vector.meshes[0].color).toEqual([0.1, 0.2, 1, 0.7])
    await expect(parseOpenSCAD('color("#abcd") cube(1);')).rejects.toThrow('Unknown color')
  })

  it('uses 2021 radius fragments, truncates explicit $fn, and lets local zero select auto mode', async () => {
    const automatic = await parseOpenSCAD('cylinder(r=10,h=1);', stable)
    const angular = await parseOpenSCAD('cylinder(r=10,h=1,$fa=30,$fs=2);', stable)
    const spatial = await parseOpenSCAD('cylinder(r=10,h=1,$fa=1,$fs=10);', stable)
    const truncated = await parseOpenSCAD('cylinder(r=10,h=1,$fn=7.9);', stable)
    const localAuto = await parseOpenSCAD('$fn=20; cylinder(r=10,h=1,$fn=0);', stable)

    // A closed n-sided Manifold cylinder contains 4n-4 triangles.
    expect(triangleCount(automatic)).toBe(4 * 30 - 4)
    expect(triangleCount(angular)).toBe(4 * 12 - 4)
    expect(triangleCount(spatial)).toBe(4 * 7 - 4)
    expect(triangleCount(truncated)).toBe(4 * 7 - 4)
    expect(triangleCount(localAuto)).toBe(4 * 30 - 4)

    const legacy = await parseOpenSCAD('cylinder(r=10,h=1);')
    expect(triangleCount(legacy)).toBe(4 * 32 - 4)
  })

  it('uses the profile radius and floor-scaled segments for partial rotate_extrude', async () => {
    const result = await parseOpenSCAD(
      'rotate_extrude(angle=90,$fa=12,$fs=2) translate([10,0]) square([1,1]);',
      stable,
    )

    // Full-circle resolution is 30 and OpenSCAD 2021 floors 30*90/360 to 7.
    expect(triangleCount(result)).toBe(60)
  })

  it('maps offset r/delta/chamfer to round, miter, and square joins', async () => {
    const miter = await parseOpenSCAD('linear_extrude(1) offset(delta=1) square(2);', stable)
    const square = await parseOpenSCAD('linear_extrude(1) offset(delta=1,chamfer=true) square(2);', stable)
    const round = await parseOpenSCAD('linear_extrude(1) offset(r=1,$fn=8) square(2);', stable)
    const fallback = await parseOpenSCAD('linear_extrude(1) offset(r="x",delta=1) square(2);', stable)

    expect(miter.volume).toBeCloseTo(16, 6)
    expect(square.volume).toBeCloseTo(15.3137085, 6)
    expect(round.volume).toBeCloseTo(14.8284271, 6)
    expect(fallback.volume).toBeCloseTo(16, 6)
  })
})
