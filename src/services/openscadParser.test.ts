import { describe, it, expect } from 'vitest'
import { parseOpenSCAD } from './openscadParser'

describe('openscadParser', () => {
  it('parses basic cube', () => {
    const meshes = parseOpenSCAD('cube([10, 5, 2]);')
    expect(meshes.length).toBe(1)
    expect(meshes[0].indices.length).toBe(36)
  })

  it('supports basic expressions in sizes', () => {
    const meshes = parseOpenSCAD('cube([1+2, 5*2, 3]);')
    expect(meshes.length).toBe(1)
    // size should be 3x10x3 but we don't check geometry here, just parses without error
  })

  it('supports parentheses in expr', () => {
    const meshes = parseOpenSCAD('cube( (1+2)*2 );')
    expect(meshes.length).toBe(1)
  })

  it('handles difference with expr', () => {
    const code = `
      difference() {
        cube(10);
        sphere(r=3+1);
      }
    `
    const meshes = parseOpenSCAD(code)
    expect(meshes.length).toBeGreaterThan(1)
  })
})