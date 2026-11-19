import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { replaceCustomizerValue } from '../src/language/scadCustomizer'
import { describe, expect, it } from 'vitest'
import { importDirectStepV9 } from '../src/kernel/cadNurbsStep'
import { exportSolidStepAssembly } from '../src/kernel/solidStepAssembly'
import { parseOpenSCAD } from '../src/language/openscadParser'
import { buildExactSolidBodies, InexactSolidError } from '../src/kernel/solid/brepBuild'

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
    const result = await evaluate(
      'brep_gear(module = 2, teeth = 12, height = 5, helix = 20, herringbone = true, bore = 6);',
    )
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

  it('exports the spinner sun after enlarging its bore with Rush subtraction', async () => {
    const source = readFileSync(new URL('../examples/rush-frontend/spinner-bore-edit.r', import.meta.url), 'utf8')
    const evaluated = await evaluate(source)
    const plan = evaluated.exactSolids!
    const bodies = buildExactSolidBodies(plan.nodes, plan.roots)
    expect(bodies).toHaveLength(1)
    const step = await exportSolidStepAssembly({ version: 1, sketches: [], bodies })
    const { model } = importDirectStepV9(step)
    expect(model.bodies).toHaveLength(1)
    expect(model.faces).toHaveLength(390)
    const minRadius = Math.min(...model.vertices.map(({ point }) => Math.hypot(point[0], point[1])))
    expect(minRadius).toBeCloseTo(22, 7)
    const fixtureDirectory = process.env.CAD_STEP_EDITED_SPINNER_FIXTURE
    if (fixtureDirectory) {
      mkdirSync(fixtureDirectory, { recursive: true })
      writeFileSync(join(fixtureDirectory, 'after.step'), step)
      const before = await evaluate(source.replace('show sun.subtract(cylinder(diameter / 2, 10mm))', 'show sun'))
      const original = before.exactSolids!
      const stock = buildExactSolidBodies(original.nodes, original.roots)
      writeFileSync(join(fixtureDirectory, 'before.step'), await exportSolidStepAssembly({
        version: 1, sketches: [], bodies: stock,
      }))
    }
  }, 60000)

  it('builds and exports the planetary spinner as twenty exact herringbone gears', async () => {
    // The spinner used to be a sampled polygon prototype; its gears are now
    // the brep_gear builtin, so the Solid workspace gets sun, ring and 18
    // planets as NURBS bodies.
    const source = readFileSync(new URL('../examples/rush-frontend/planetary-spinner.r', import.meta.url), 'utf8')
    const result = await evaluate(source.replace('param hole = 44mm', 'param hole = 42mm'))
    expect(result.meshes.length).toBeGreaterThan(0)
    const plan = result.exactSolids!
    expect(plan.roots).toHaveLength(20)
    expect(plan.nodes.filter(node => node.op === 'brep_gear')).toHaveLength(3)
    const byId = new Map(plan.nodes.map(node => [node.id, node]))
    const prototypes = plan.roots.slice(2).map(root => {
      if (!('id' in root)) throw Error('Expected an exact planet')
      let node = byId.get(root.id)
      while (node?.op === 'transform') node = byId.get(String(node.input))
      expect(node?.teeth).toBe(4)
      return node?.id
    })
    expect(new Set(prototypes).size).toBe(1)
    const bodies = buildExactSolidBodies(plan.nodes, plan.roots)
    expect(bodies).toHaveLength(20)
    expect(bodies.every(b => b.brep && b.brep.faces.length > 8)).toBe(true)
    const step = await exportSolidStepAssembly({ version: 1, sketches: [], bodies })
    const definitions = step.match(/=MANIFOLD_SOLID_BREP\(/g)?.length ?? 0
    const occurrences = step.match(/=NEXT_ASSEMBLY_USAGE_OCCURRENCE\(/g)?.length ?? 0
    expect(occurrences).toBe(20)
    expect(definitions).toBe(3)
    const prototype = bodies[2]
    for (const planet of bodies.slice(3)) {
      expect(planet.instance?.sourceId).toBe(prototype.id)
      const matrix = planet.instance!.matrix
      const point = prototype.brep!.vertices[0].point
      const placed = matrix.slice(0, 3).map(row => row[3] + point.reduce((sum, x, i) => sum + x * row[i], 0))
      placed.forEach((x, i) => expect(x).toBeCloseTo(planet.brep!.vertices[0].point[i], 9))
    }
    const grouped = await exportSolidStepAssembly({
      version: 1,
      sketches: [],
      bodies: bodies.map((body, i) => ({ ...body, group: i < 2 ? 'Central gears' : 'Planets' })),
    })
    expect(grouped.match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(3)
    expect(grouped.match(/=NEXT_ASSEMBLY_USAGE_OCCURRENCE\(/g)).toHaveLength(22)
    expect(grouped).toContain("PRODUCT('group:Planets','Planets'")
    expect(grouped).toContain("PRODUCT('group:Central gears','Central gears'")
    const styled = await exportSolidStepAssembly({
      version: 1, sketches: [],
      bodies: bodies.map((body, i) => ({
        ...body,
        group: i < 2 ? 'Central gears' : 'Planets',
        material: i % 2 ? { name: 'Red PLA', color: '#ff0000' } : { name: 'Blue PLA', color: '#0000ff' },
      })),
    })
    expect(styled.match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(3)
    expect(styled.match(/=NEXT_ASSEMBLY_USAGE_OCCURRENCE\(/g)).toHaveLength(22)
    expect(styled.match(/=PRESENTATION_STYLE_BY_CONTEXT\(/g)).toHaveLength(20)
    expect(styled.match(/=COLOUR_RGB\('Red PLA',/g)).toHaveLength(10)
    expect(styled.match(/=COLOUR_RGB\('Blue PLA',/g)).toHaveLength(10)
    const fixtureDirectory = process.env.CAD_STEP_STYLED_SPINNER_FIXTURE
    if (fixtureDirectory) {
      mkdirSync(fixtureDirectory, { recursive: true })
      writeFileSync(join(fixtureDirectory, 'spinner.step'), styled)
      const manifest = bodies.map((body, i) => ({
        id: body.id, name: body.name, colour: i % 2 ? 'red' : 'blue', xyz: body.brep!.vertices[0].point,
      }))
      writeFileSync(join(fixtureDirectory, 'bodies.json'), JSON.stringify(manifest))
    }
    console.info('Rush spinner STEP', { definitions, occurrences, bytes: Buffer.byteLength(step) })
  }, 600000)

  it('rebuilds the spinner assembly for both customizer bore settings', async () => {
    let source = readFileSync(new URL('../examples/rush/planetary-spinner.r', import.meta.url), 'utf8')
    for (const diameter of [42, 44]) {
      const hole = compileRushFrontend(source).customizer.find(parameter => parameter.name === 'hole')!
      source = replaceCustomizerValue(source, hole, diameter)
      const { exactSolids } = await evaluate(source)
      const bodies = buildExactSolidBodies(exactSolids!.nodes, exactSolids!.roots)
      const radius = Math.min(...bodies[0].brep!.vertices.map(({ point }) => Math.hypot(point[0], point[1])))
      expect(radius).toBeCloseTo(diameter / 2, 7)
      const step = await exportSolidStepAssembly({ version: 1, sketches: [], bodies })
      expect(step.match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(3)
      expect(step.match(/=NEXT_ASSEMBLY_USAGE_OCCURRENCE\(/g)).toHaveLength(20)
      const directory = process.env.CAD_STEP_SPINNER_BORE_ASSEMBLIES
      if (directory) {
        mkdirSync(directory, { recursive: true })
        writeFileSync(join(directory, `spinner-${diameter}.step`), step)
      }
    }
  }, 600000)

  it('lowers a helical RushGraph gear through the same builtin', async () => {
    const result = await evaluate(
      [
        '// @rush/1',
        'wheel = gear(teeth: 24, module: 1.5mm, thickness: 8mm, helix_angle: 30deg, herringbone: true, bore: 5mm)',
        'show wheel',
      ].join('\n'),
    )
    const plan = result.exactSolids!
    const [body] = buildExactSolidBodies(plan.nodes, plan.roots)
    expect(body.brep!.faces).toHaveLength((24 * 6 + 12) * 2 + 2)
  }, 60000)

  it('records the second language too, since it compiles through the same evaluator', async () => {
    const result = await evaluate(['// @rush/1', 'part = box(10mm, 10mm, 10mm)', 'show part'].join('\n'))
    const plan = result.exactSolids!
    const [body] = buildExactSolidBodies(plan.nodes, plan.roots)
    expect(body.brep!.faces).toHaveLength(6)
    expect(bounds(body.mesh.positions).max).toEqual([10, 10, 10])
  })
})
