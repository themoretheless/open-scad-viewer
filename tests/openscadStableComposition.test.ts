import { describe, expect, it } from 'vitest'
import { parseOpenSCAD } from '../src/services/openscadParser'

const stable = { languageProfile: 'openscad/stable-2021.01' as const }

describe('OpenSCAD 2021.01 independent geometry composition', () => {
  it('applies implicit union at root, group, render, and user-module boundaries', async () => {
    for (const source of [
      'cube(2); translate([1,0,0]) cube(2);',
      'group() { cube(2); translate([1,0,0]) cube(2); }',
      'render(convexity=3) { cube(2); translate([1,0,0]) cube(2); }',
      'module m() { cube(2); translate([1,0,0]) cube(2); } m();',
    ]) {
      const result = await parseOpenSCAD(source, stable)
      expect(result.meshes, source).toHaveLength(1)
      expect(result.volume, source).toBeCloseTo(12, 6)
    }
  })

  it('uses first-child dimension arbitration instead of failing mixed CSG', async () => {
    const union3d = await parseOpenSCAD('union(){ cube(1); square(2); }', stable)
    expect(union3d.meshes).toHaveLength(1)
    expect(union3d.volume).toBeCloseTo(1, 6)
    expect(union3d.warnings).toContain('union() ignored child geometry with a different dimension')

    const difference3d = await parseOpenSCAD('difference(){ cube(1); square(2); }', stable)
    expect(difference3d.volume).toBeCloseTo(1, 6)
    expect(difference3d.warnings).toContain('difference() ignored child geometry with a different dimension')

    const intersection = await parseOpenSCAD('intersection(){ cube(1); square(2); }', stable)
    expect(intersection.meshes).toEqual([])
  })

  it('unions projection output and ignores non-3D children in cut and flat modes', async () => {
    for (const cut of ['false', 'true']) {
      const result = await parseOpenSCAD(`
        linear_extrude(height=1)
          projection(cut=${cut}, convexity=4) {
            cube(1);
            translate([0.5,0,0]) cube(1);
            square(2);
          }
      `, stable)
      expect(result.meshes, cut).toHaveLength(1)
      expect(result.volume, cut).toBeCloseTo(1.5, 6)
      expect(result.warnings).toContain('projection() ignored non-3D child geometry')
    }
  })

  it('convexifies a single concave hull child instead of treating hull as identity', async () => {
    const result = await parseOpenSCAD(`
      linear_extrude(height=1)
        hull()
          polygon([[0,0],[2,0],[2,1],[1,1],[1,2],[0,2]]);
    `, stable)

    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(3.5, 6)
  })
})
