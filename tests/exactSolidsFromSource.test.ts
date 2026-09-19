import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { buildExactSolidBodies, InexactSolidError } from '../src/services/solid/brepBuild'

/** Both workspaces are fed by one evaluation: meshes for Mesh, the plan for Solid. */
async function evaluate(source: string) {
  const result = await parseOpenSCAD(source, { recordExactSolids: true })
  expect(result.exactSolids).toBeDefined()
  return result
}

function bounds(positions: readonly number[]) {
  const min = [Infinity, Infinity, Infinity]
  const max = [-Infinity, -Infinity, -Infinity]
  for (let index = 0; index < positions.length; index++) {
    const axis = index % 3
    min[axis] = Math.min(min[axis], positions[index])
    max[axis] = Math.max(max[axis], positions[index])
  }
  return { min, max }
}

describe('exact solids from source', () => {
  it('builds an OpenSCAD cube as a true B-rep body and as a mesh', async () => {
    const result = await evaluate('cube([10, 20, 30], center = true);')
    expect(result.meshes).toHaveLength(1)

    const plan = result.exactSolids!
    const bodies = buildExactSolidBodies(plan.nodes, plan.roots)
    expect(bodies).toHaveLength(1)
    expect(bodies[0].brep!.faces).toHaveLength(6)
    expect(bounds(bodies[0].mesh.positions).max).toEqual([5, 10, 15])
  })

  it('carries a transform from the source into the exact body', async () => {
    const result = await evaluate('translate([10, 0, 0]) cube([2, 2, 2]);')
    const plan = result.exactSolids!
    const [body] = buildExactSolidBodies(plan.nodes, plan.roots)
    const extent = bounds(body.mesh.positions)
    expect(extent.min[0]).toBeCloseTo(10, 6)
    expect(extent.max[0]).toBeCloseTo(12, 6)
  })

  it('keeps several top-level objects as separate bodies', async () => {
    const result = await evaluate('cube([2, 2, 2]); translate([8, 0, 0]) cube([2, 2, 2]);')
    const plan = result.exactSolids!
    expect(plan.roots).toHaveLength(2)
    expect(buildExactSolidBodies(plan.nodes, plan.roots)).toHaveLength(2)
  })

  it('refuses a hull by name rather than approximating it', async () => {
    const result = await evaluate('hull() { cube([2, 2, 2]); translate([6, 0, 0]) cube([2, 2, 2]); }')
    // The polygon track still produced geometry, so the same source opens in Mesh.
    expect(result.meshes).toHaveLength(1)
    const plan = result.exactSolids!
    expect(() => buildExactSolidBodies(plan.nodes, plan.roots)).toThrow(InexactSolidError)
    expect(() => buildExactSolidBodies(plan.nodes, plan.roots)).toThrow(/hull/)
  })

  it('builds an involute gear from the brep_gear builtin as one exact body with six faces per tooth', async () => {
    const result = await evaluate('brep_gear(module = 2, teeth = 12, height = 5, helix = 20, herringbone = true, bore = 6);')
    expect(result.meshes).toHaveLength(1)
    const plan = result.exactSolids!
    const [body] = buildExactSolidBodies(plan.nodes, plan.roots)
    // 12 teeth x 6 outline curves + 6 bore arcs (one per two teeth), twice for the
    // herringbone halves, plus two caps.
    expect(body.brep!.faces).toHaveLength((12 * 6 + 6) * 2 + 2)
    const extent = bounds(body.mesh.positions)
    expect(extent.max[2]).toBeCloseTo(5, 6)
    expect(extent.max[0]).toBeCloseTo(14, 1)
  }, 30000)

  it('builds the planetary spinner as twenty exact herringbone gears', async () => {
    // The spinner used to be a sampled polygon prototype; its gears are now
    // the brep_gear builtin, so the Solid workspace gets sun, ring and 18
    // planets as NURBS bodies.
    const source = readFileSync(new URL('../examples/modelgraph-text/planetary-spinner.mg', import.meta.url), 'utf8')
    const result = await evaluate(source)
    expect(result.meshes.length).toBeGreaterThan(0)
    const plan = result.exactSolids!
    expect(plan.roots).toHaveLength(20)
    const bodies = buildExactSolidBodies(plan.nodes, plan.roots)
    expect(bodies).toHaveLength(20)
    expect(bodies.every(b => b.brep && b.brep.faces.length > 8)).toBe(true)
  }, 120000)

  it('lowers a helical ModelGraph gear through the same builtin', async () => {
    const result = await evaluate([
      '// @modelgraph-text/1',
      'wheel = gear(teeth: 24, module: 1.5mm, thickness: 8mm, helix_angle: 30deg, herringbone: true, bore: 5mm)',
      'show wheel',
    ].join('\n'))
    const plan = result.exactSolids!
    const [body] = buildExactSolidBodies(plan.nodes, plan.roots)
    expect(body.brep!.faces).toHaveLength((24 * 6 + 12) * 2 + 2)
  }, 60000)

  it('records the second language too, since it compiles through the same evaluator', async () => {
    const result = await evaluate([
      '// @modelgraph-text/1',
      'part = box(10mm, 10mm, 10mm)',
      'show part',
    ].join('\n'))
    const plan = result.exactSolids!
    const [body] = buildExactSolidBodies(plan.nodes, plan.roots)
    expect(body.brep!.faces).toHaveLength(6)
    expect(bounds(body.mesh.positions).max).toEqual([10, 10, 10])
  })
})
