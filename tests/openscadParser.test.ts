import { describe, expect, it } from 'vitest'
import { OpenSCADParseError, parseOpenSCAD, type MeshData } from '../src/services/openscadParser'
import { EXAMPLES } from '../src/data/examples'

function bounds(meshes: MeshData[]) {
  const min = [Infinity, Infinity, Infinity]
  const max = [-Infinity, -Infinity, -Infinity]
  for (const mesh of meshes) {
    for (let i = 0; i < mesh.vertices.length; i += 6) {
      for (let axis = 0; axis < 3; axis++) {
        min[axis] = Math.min(min[axis], mesh.vertices[i + axis])
        max[axis] = Math.max(max[axis], mesh.vertices[i + axis])
      }
    }
  }
  return { min, max }
}

describe('OpenSCAD parser and Manifold evaluator', () => {
  it('evaluates variables and arithmetic', async () => {
    const result = await parseOpenSCAD('x = 10; cube([x, 2 + 3, 4]);')
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(200, 5)
    expect(bounds(result.meshes)).toEqual({ min: [0, 0, 0], max: [10, 5, 4] })
  })

  it('performs real boolean difference', async () => {
    const result = await parseOpenSCAD(`
      difference() {
        cube([10, 10, 10], center = true);
        sphere(r = 3, $fn = 48);
      }
    `)
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeGreaterThan(885)
    expect(result.volume).toBeLessThan(890)
    expect(result.meshes[0].color[3]).toBe(1)
  })

  it('composes nested transforms as parent * local', async () => {
    const result = await parseOpenSCAD('translate([10, 0, 0]) rotate([0, 0, 90]) cube([2, 4, 6]);')
    const box = bounds(result.meshes)
    expect(box.min[0]).toBeCloseTo(6, 5)
    expect(box.max[0]).toBeCloseTo(10, 5)
    expect(box.min[1]).toBeCloseTo(0, 5)
    expect(box.max[1]).toBeCloseTo(2, 5)
  })

  it('supports modules, ranges and loops', async () => {
    const result = await parseOpenSCAD(`
      module peg(x = 0) { translate([x, 0, 0]) cylinder(h = 2, r = 1, $fn = 16); }
      for (i = [0:2:4]) peg(i);
    `)
    expect(result.meshes).toHaveLength(3)
    expect(result.volume).toBeGreaterThan(18)
    expect(bounds(result.meshes).max[0]).toBeCloseTo(5, 5)
  })

  it('supports 2D boolean geometry and extrusion', async () => {
    const result = await parseOpenSCAD(`
      linear_extrude(height = 5)
        difference() {
          square([10, 10], center = true);
          circle(r = 2, $fn = 48);
        }
    `)
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeGreaterThan(435)
    expect(result.volume).toBeLessThan(440)
  })

  it('attributes a generated 3D hull to the hull source call', async () => {
    const source = `hull() {
      cube([2, 2, 2], center = true);
      translate([8, 0, 0]) sphere(r = 2, $fn = 16);
    }`
    const result = await parseOpenSCAD(source)
    const runs = result.meshes[0].provenance

    expect(runs.length).toBeGreaterThan(0)
    expect(runs.every(run => run.source?.label === 'hull()')).toBe(true)
    expect(runs.every(run => run.source?.start === source.indexOf('hull'))).toBe(true)
    expect(runs.every(run => run.source?.end === source.length)).toBe(true)
  })

  it('reports unsupported syntax with line and column', async () => {
    await expect(parseOpenSCAD('cube(1);\ntext("nope");')).rejects.toMatchObject({
      name: 'OpenSCADParseError',
      line: 2,
      column: 1,
    } satisfies Partial<OpenSCADParseError>)
  })

  it('clamps excessive tessellation', async () => {
    const result = await parseOpenSCAD('sphere(1, $fn = 9999);')
    expect(result.warnings[0]).toContain('clamped')
    expect(result.meshes[0].indices.length / 3).toBeLessThan(MAX_SAFE_TEST_TRIANGLES)
  })

  it('renders every bundled example', async () => {
    for (const [name, source] of Object.entries(EXAMPLES)) {
      const result = await parseOpenSCAD(source)
      expect(result.meshes.length, name).toBeGreaterThan(0)
      expect(result.meshes.every(mesh => mesh.vertices.every(Number.isFinite)), name).toBe(true)
    }
  })

  it('builds OpenSCAD-convention (clockwise-from-outside) polyhedra outward-facing', async () => {
    // Unit tetrahedron with faces wound per the OpenSCAD spec; volume must be
    // positive ~1/6 (the inverted-winding bug rejected or everted it).
    const result = await parseOpenSCAD(
      'polyhedron(points=[[0,0,0],[1,0,0],[0,1,0],[0,0,1]], faces=[[0,1,2],[0,2,3],[0,3,1],[1,3,2]]);',
    )
    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(1 / 6, 3)
  })

  it('accepts clockwise-wound polygon() point lists', async () => {
    // With the default Positive fill rule a CW outline yields an EMPTY shape.
    const result = await parseOpenSCAD('linear_extrude(height=2) polygon(points=[[0,0],[0,4],[4,4],[4,0]]);')
    expect(result.volume).toBeCloseTo(32, 3)
  })

  it('bounds nested loops that produce no geometry', async () => {
    const started = Date.now()
    await expect(parseOpenSCAD('for(i=[0:9999]) for(j=[0:9999]) x = i + j;')).rejects.toThrow(/evaluation step limit/)
    expect(Date.now() - started).toBeLessThan(10_000)
  })

  it('caps runaway concat() growth', async () => {
    const doubling = Array.from({ length: 40 }, () => 'a = concat(a, a);').join('\n')
    await expect(parseOpenSCAD(`a = [0:1:999];\n${doubling}\ncube(1);`)).rejects.toThrow(/concat\(\) result exceeds/)
  })

  it('caps linear_extrude slices before they reach the kernel', async () => {
    const started = Date.now()
    const result = await parseOpenSCAD('linear_extrude(height=10, twist=90, slices=100000000) square(5);')
    expect(result.meshes.length).toBeGreaterThan(0)
    expect(Date.now() - started).toBeLessThan(15_000)
  })
})

const MAX_SAFE_TEST_TRIANGLES = 150_000
