import { beforeAll, describe, expect, it } from 'vitest'
import { GeometryBuildEngine } from '../src/kernel/geometryBuildEngine'
import { extractSemanticEdges } from '../src/kernel/meshTopology'
import { generateVoronoiCells, type VoronoiCellsOptions } from '../src/kernel/geometry/voronoi'
import { parseOpenSCAD } from '../src/language/openscadParser'
import { extractCustomizerParameters } from '../src/language/scadCustomizer'
import {
  bboxOfPoly,
  emitCoupledVoronoiFloor,
  generateTrabecularCells,
  signedArea,
} from '../src/language/trabecularLattice'
import {
  generateSkadisVoronoiBox,
  SKADIS_VORONOI_BOX_DEFAULTS,
  type SkadisVoronoiBoxParams,
} from '../src/app/skadisVoronoiBox'

const engine = new GeometryBuildEngine()
const request = { quality: 'full', purpose: 'analysis' } as const

function expectClosedManifold(meshes: Awaited<ReturnType<typeof engine.buildSource>>['result']['meshes']) {
  expect(meshes.length).toBeGreaterThanOrEqual(1)
  for (const mesh of meshes) {
    const { diagnostics } = extractSemanticEdges(mesh.vertices, mesh.indices)
    expect(diagnostics.boundary).toBe(0)
    expect(diagnostics.nonManifold).toBe(0)
  }
}

function auditOverhangs(
  meshes: Awaited<ReturnType<typeof engine.buildSource>>['result']['meshes'],
  limitDeg: number,
  ignoreFace?: (cx: number, cy: number, cz: number) => boolean,
) {
  let worst = 0
  let criticalWorst = 0
  let excessArea = 0
  for (const mesh of meshes) {
    const v = mesh.vertices
    for (let i = 0; i < mesh.indices.length; i += 3) {
      const a = mesh.indices[i]! * 6
      const b = mesh.indices[i + 1]! * 6
      const c = mesh.indices[i + 2]! * 6
      if (Math.max(v[a + 2]!, v[b + 2]!, v[c + 2]!) < 0.001) continue
      const ux = v[b]! - v[a]!
      const uy = v[b + 1]! - v[a + 1]!
      const uz = v[b + 2]! - v[a + 2]!
      const vx = v[c]! - v[a]!
      const vy = v[c + 1]! - v[a + 1]!
      const vz = v[c + 2]! - v[a + 2]!
      const nx = uy * vz - uz * vy
      const ny = uz * vx - ux * vz
      const nz = ux * vy - uy * vx
      const size = Math.hypot(nx, ny, nz)
      if (nz >= -1e-8 || size < 1e-6) continue
      const cx = (v[a]! + v[b]! + v[c]!) / 3
      const cy = (v[a + 1]! + v[b + 1]! + v[c + 1]!) / 3
      const cz = (v[a + 2]! + v[b + 2]! + v[c + 2]!) / 3
      if (ignoreFace?.(cx, cy, cz)) continue
      const angle = (Math.asin(Math.min(1, -nz / size)) * 180) / Math.PI
      worst = Math.max(worst, angle)
      if (angle > limitDeg + 0.2) {
        const maxEdge = Math.max(
          Math.hypot(ux, uy, uz),
          Math.hypot(vx, vy, vz),
          Math.hypot(v[c]! - v[b]!, v[c + 1]! - v[b + 1]!, v[c + 2]! - v[b + 2]!),
        )
        const minWidth = size / maxEdge
        if (minWidth > 0.4) {
          criticalWorst = Math.max(criticalWorst, angle)
          excessArea += size / 2
        }
      }
    }
  }
  return { worst, criticalWorst, excessArea }
}

describe('unified SKADIS + Voronoi box mechanism', () => {
  beforeAll(async () => {
    await engine.initializeSource('cube(1);', request)
  }, 120000)

  it('exposes the unified Rust cad_voronoi_cells engine across all patterns, overhangs, gradients and keep-outs', () => {
    const base: VoronoiCellsOptions = {
      min: [0, 0],
      max: [100, 70],
      pattern: 'voronoi',
      cell: 16,
      rib: 2.4,
      seed: 1,
      jitter: 0.7,
      frame: 3,
    }
    const voronoi = generateVoronoiCells(base)
    expect(voronoi).toEqual(generateVoronoiCells({ ...base, pattern: 'web' }))
    expect(voronoi.length).toBeGreaterThanOrEqual(8)
    for (const pattern of ['honeycomb', 'isogrid', 'triangles', 'grid'] as const) {
      expect(generateVoronoiCells({ ...base, pattern }).length).toBeGreaterThan(0)
    }

    for (const overhangAngle of [35, 45]) {
      const printable = generateVoronoiCells({ ...base, overhangAngle })
      expect(printable.length).toBeGreaterThan(0)
      for (const poly of printable) {
        for (let i = 0; i < poly.length; i++) {
          const a = poly[i]!
          const b = poly[(i + 1) % poly.length]!
          const dx = b[0] - a[0]
          const dy = b[1] - a[1]
          const len = Math.hypot(dx, dy)
          if (len < 1e-6) continue
          const ny = -dx / len
          if (ny > 1e-5) {
            const angle = (Math.asin(Math.min(1, ny)) * 180) / Math.PI
            expect(angle).toBeLessThanOrEqual(overhangAngle + 1e-3)
          }
        }
      }
    }

    const vert = generateVoronoiCells({ ...base, stressGradient: 1.5, gradientMode: 'vertical' })
    const rad = generateVoronoiCells({
      ...base,
      stressGradient: 1.2,
      gradientMode: 'radial',
      cornerRadius: 2,
      slope: 0.2,
      keepOut: [[35, 25, 65, 70]],
    })
    expect(vert.length).toBeGreaterThan(0)
    expect(rad.length).toBeGreaterThan(0)
    for (const poly of rad) {
      const insideKeepOut = poly.some(([x, y]) => x > 35.01 && x < 64.99 && y > 25.01 && y < 69.99)
      expect(insideKeepOut).toBe(false)
    }
    expect(generateVoronoiCells({ ...base, min: [0, 0], max: [8, 70], frame: 3.5 })).toEqual([])
  })

  it('supports native OpenSCAD viewer_voronoi() extrusion and handles empty domains cleanly', async () => {
    const plate = `linear_extrude(3) difference() {
      square([100, 70]);
      viewer_voronoi(min=[0,0], max=[100,70], cell=16, rib=2.4, pattern="voronoi", overhang=45, frame=3);
    }`
    const { result } = await engine.buildSource(plate, request)
    expectClosedManifold(result.meshes)
    expect(result.volume).toBeGreaterThan(0)
    expect(result.volume).toBeLessThan(100 * 70 * 3 * 0.6)

    const narrow = `linear_extrude(3) difference() {
      square([8, 40]);
      viewer_voronoi(size=[8, 40], cell=16, rib=2.4, frame=3.5);
    }`
    const narrowRes = await engine.buildSource(narrow, request)
    expectClosedManifold(narrowRes.result.meshes)
    expect(narrowRes.result.volume).toBeCloseTo(8 * 40 * 3, 2)

    await expect(parseOpenSCAD('linear_extrude(2) viewer_voronoi(size=[40,40], cell=0, rib=2);')).rejects.toThrow()
    await expect(
      parseOpenSCAD('linear_extrude(2) viewer_voronoi(size=[40,40], cell=16, rib=2);', {
        languageProfile: 'openscad/stable-2021.01',
      }),
    ).rejects.toThrow(/viewer_voronoi/)

    const smoothOffset = await engine.buildSource(
      'linear_extrude(2) { $fn = 48; offset(r = 5) square([20, 20]); }',
      request,
    )
    expectClosedManifold(smoothOffset.result.meshes)
    const offsetTopVerts = new Set<string>()
    for (const m of smoothOffset.result.meshes) {
      for (let i = 0; i < m.vertices.length; i += 6) {
        if (Math.abs(m.vertices[i + 2]! - 2) < 0.05) {
          offsetTopVerts.add(`${m.vertices[i]!.toFixed(2)},${m.vertices[i + 1]!.toFixed(2)}`)
        }
      }
    }
    expect(offsetTopVerts.size).toBeGreaterThanOrEqual(40)
  }, 120000)

  it('builds IKEA SKADIS dovetail + Voronoi in all 4 part modes with exact 40 mm pitch and flat printable hooks', async () => {
    const solidRes = await engine.buildSource(
      generateSkadisVoronoiBox({
        ...SKADIS_VORONOI_BOX_DEFAULTS,
        lattice_mode: 'solid',
        floor_mesh: false,
        skadis_part: 1,
      }),
      request,
    )
    const bodyRes = await engine.buildSource(
      generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, skadis_part: 1 }),
      request,
    )
    expectClosedManifold(bodyRes.result.meshes)
    expect(bodyRes.result.volume).toBeGreaterThan(0)
    expect(bodyRes.result.volume).toBeLessThan(solidRes.result.volume * 0.65)

    const hookRes = await engine.buildSource(
      generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, skadis_part: 2 }),
      request,
    )
    expectClosedManifold(hookRes.result.meshes)
    const hookAudit = auditOverhangs(hookRes.result.meshes, 45)
    expect(hookAudit.excessArea).toBe(0)
    let minZ = Infinity
    let maxZ = -Infinity
    for (const m of hookRes.result.meshes) {
      for (let i = 0; i < m.vertices.length; i += 6) {
        minZ = Math.min(minZ, m.vertices[i + 2]!)
        maxZ = Math.max(maxZ, m.vertices[i + 2]!)
      }
    }
    expect(minZ).toBeCloseTo(0, 2)
    expect(maxZ).toBeCloseTo(8.4, 2)

    const plateRes = await engine.buildSource(
      generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, skadis_part: 3 }),
      request,
    )
    expectClosedManifold(plateRes.result.meshes)
    let plateNegYMinZ = Infinity
    for (const m of plateRes.result.meshes) {
      for (let i = 0; i < m.vertices.length; i += 6) {
        if (m.vertices[i + 1]! < -10) plateNegYMinZ = Math.min(plateNegYMinZ, m.vertices[i + 2]!)
      }
    }
    expect(plateNegYMinZ).toBeCloseTo(0, 2)

    // Assembly check: two hooks spaced 80 mm apart (2 * 40 mm SKADIS pitch), 4.4 mm wide, behind 5 mm board
    const asmRes = await engine.buildSource(
      generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, skadis_part: 0 }),
      request,
    )
    expectClosedManifold(asmRes.result.meshes)
    expect(asmRes.result.meshes.length).toBeGreaterThanOrEqual(3)
    const leftXs: number[] = []
    const rightXs: number[] = []
    let minHookY = Infinity
    for (const hookMesh of asmRes.result.meshes) {
      for (let i = 0; i < hookMesh.vertices.length; i += 6) {
        const x = hookMesh.vertices[i]!
        const y = hookMesh.vertices[i + 1]!
        minHookY = Math.min(minHookY, y)
        if (y < -5) {
          if (x < 60) leftXs.push(x)
          else rightXs.push(x)
        }
      }
    }
    expect(minHookY).toBeLessThan(-8)
    expect(Math.max(...leftXs) - Math.min(...leftXs)).toBeCloseTo(4.4, 2)
    const leftCenter = (Math.min(...leftXs) + Math.max(...leftXs)) / 2
    const rightCenter = (Math.min(...rightXs) + Math.max(...rightXs)) / 2
    expect(rightCenter - leftCenter).toBeCloseTo(80, 2)
  }, 120000)

  it.each([
    { overhang_angle: 45, front_angle: 0, corner_radius: 5, lattice_mode: 'voronoi' as const },
    { overhang_angle: 35, front_angle: 12, corner_radius: 6, lattice_mode: 'voronoi' as const },
    { overhang_angle: 45, front_angle: 0, corner_radius: 5, lattice_mode: 'trabecular' as const },
    { overhang_angle: 35, front_angle: 12, corner_radius: 6, lattice_mode: 'trabecular' as const },
  ])(
    'keeps Voronoi wall overhangs strictly inside limit %j',
    async patch => {
      const p: SkadisVoronoiBoxParams = {
        ...SKADIS_VORONOI_BOX_DEFAULTS,
        mount_system: 'none',
        print_optimized: true,
        ...patch,
      }
      const { result } = await engine.buildSource(generateSkadisVoronoiBox(p), request)
      expectClosedManifold(result.meshes)
      const report = auditOverhangs(result.meshes, patch.overhang_angle)
      expect(report.excessArea).toBeLessThan(0.02)
      expect(report.criticalWorst).toBeLessThanOrEqual(patch.overhang_angle + 0.2)
      if (patch.front_angle !== 0 || patch.corner_radius < SKADIS_VORONOI_BOX_DEFAULTS.wall + 1.2) {
        expect(report.worst).toBeLessThanOrEqual(patch.overhang_angle + 0.2)
      }
    },
    120000,
  )

  it.each(['trabecular', 'voronoi'] as const)(
    'keeps SKADIS dovetail %s walls overhang-safe outside the 5 mm channel throat bridges',
    async lattice_mode => {
      const p: SkadisVoronoiBoxParams = {
        ...SKADIS_VORONOI_BOX_DEFAULTS,
        lattice_mode,
        print_optimized: true,
        skadis_part: 1,
        front_angle: 12,
        corner_radius: 6,
      }
      const { result } = await engine.buildSource(generateSkadisVoronoiBox(p), request)
      expectClosedManifold(result.meshes)
      const report = auditOverhangs(result.meshes, 45, (cx, cy, cz) => {
        const inThroatZ = cz > 64 && cz < 68
        const inThroatY = cy >= -0.1 && cy <= 3.5
        const nearHookX = Math.abs(cx - 20) <= 3 || Math.abs(cx - 100) <= 3
        return inThroatZ && inThroatY && nearHookX
      })
      expect(report.excessArea).toBeLessThan(0.02)
    },
    120000,
  )

  it('supports SKADIS integral, honeycomb, isogrid, rail dovetail, and curved Voronoi boxes', async () => {
    const variants: Array<Partial<SkadisVoronoiBoxParams>> = [
      { mount_system: 'skadis_integral', hook_chamfer: true },
      { lattice_mode: 'honeycomb', skadis_part: 1 },
      { lattice_mode: 'isogrid', skadis_part: 1 },
      { mount_system: 'rail_dovetail', front_angle: 10 },
      { mount_system: 'rail_monolithic', bend_radius: 160, corner_radius: 6 },
    ]
    for (const patch of variants) {
      const { result } = await engine.buildSource(
        generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, ...patch }),
        request,
      )
      expect(result.volume).toBeGreaterThan(0)
      expectClosedManifold(result.meshes)
    }
  }, 120000)

  it('supports independent wall and floor infill combinations and exposes wall_fill / floor_fill customizer dropdowns', async () => {
    const scad = generateSkadisVoronoiBox({
      ...SKADIS_VORONOI_BOX_DEFAULTS,
      lattice_mode: 'voronoi',
      floor_lattice_mode: 'honeycomb',
      skadis_part: 1,
    })
    const params = extractCustomizerParameters(scad)
    const wallParam = params.find(p => p.name === 'wall_fill')
    const floorParam = params.find(p => p.name === 'floor_fill')
    expect(wallParam?.value).toBe('voronoi')
    expect(wallParam?.options).toEqual([
      'trabecular',
      'voronoi',
      'honeycomb',
      'isogrid',
      'triangles',
      'grid',
      'circles',
      'solid',
    ])
    expect(floorParam?.value).toBe('honeycomb')
    expect(floorParam?.options).toEqual([
      'trabecular',
      'voronoi',
      'honeycomb',
      'isogrid',
      'triangles',
      'grid',
      'circles',
      'solid',
    ])

    const meshFloor = await engine.buildSource(scad, request)
    expectClosedManifold(meshFloor.result.meshes)

    const solidFloor = await engine.buildSource(
      generateSkadisVoronoiBox({
        ...SKADIS_VORONOI_BOX_DEFAULTS,
        lattice_mode: 'voronoi',
        floor_lattice_mode: 'solid',
        skadis_part: 1,
      }),
      request,
    )
    expectClosedManifold(solidFloor.result.meshes)
    expect(solidFloor.result.volume).toBeGreaterThan(meshFloor.result.volume + 5000)

    for (const [lattice_mode, floor_lattice_mode] of [
      ['solid', 'isogrid'],
      ['triangles', 'grid'],
      ['honeycomb', 'circles'],
    ] as const) {
      const mixed = await engine.buildSource(
        generateSkadisVoronoiBox({
          ...SKADIS_VORONOI_BOX_DEFAULTS,
          lattice_mode,
          floor_lattice_mode,
          skadis_part: 1,
        }),
        request,
      )
      expectClosedManifold(mixed.result.meshes)
      expect(mixed.result.volume).toBeGreaterThan(0)
    }
  }, 120000)

  it('eliminates inner bosses/floor ledges/bottom plinth and smoothly rounds cells even with print_optimized + corner_radius', async () => {
    const p: SkadisVoronoiBoxParams = {
      ...SKADIS_VORONOI_BOX_DEFAULTS,
      skadis_part: 1,
      lattice_mode: 'voronoi',
      cell_size: 16,
      floor_cell_size: 16,
      cell_radius: 2,
      floor_cell_radius: 2,
      floor_lattice_mode: 'honeycomb',
      print_optimized: true,
      corner_radius: 5,
    }
    const sharp = await engine.buildSource(
      generateSkadisVoronoiBox({ ...p, cell_radius: 0, floor_cell_radius: 0 }),
      request,
    )
    const rounded = await engine.buildSource(generateSkadisVoronoiBox(p), request)
    expectClosedManifold(sharp.result.meshes)
    expectClosedManifold(rounded.result.meshes)

    const sharpTris = sharp.result.meshes.reduce((s, m) => s + m.indices.length / 3, 0)
    const roundedTris = rounded.result.meshes.reduce((s, m) => s + m.indices.length / 3, 0)
    expect(roundedTris).toBeGreaterThan(sharpTris + 100)
    expect(rounded.result.volume).toBeGreaterThan(sharp.result.volume + 50)

    const audit = auditOverhangs(rounded.result.meshes, 45, (cx, cy, cz) => {
      const inThroatZ = cz > 64 && cz < 68
      const inThroatY = cy >= -0.1 && cy <= 3.5
      const nearHookX = Math.abs(cx - 20) <= 3 || Math.abs(cx - 100) <= 3
      return inThroatZ && inThroatY && nearHookX
    })
    expect(audit.excessArea).toBeLessThan(0.02)

    // 1. No protruding inward back-wall bosses past slot strips anywhere in the inner cavity,
    //    AND the back wall bays between dovetail strips match side/front wall thickness (p.wall = 3 mm)
    let cavityIntruder = 0
    let thickBayIntruder = 0
    // 2. Floor cutouts flush up against back wall (y = p.wall) and front inner wall
    let minFloorHoleY = Infinity
    let maxFloorHoleY = -Infinity
    // 3. Wall cutouts flush down to z = floor (no bottom plinth step)
    let hasFlushBottomWallOpening = false
    const innerR = Math.max(0, p.corner_radius - p.wall)
    for (const m of rounded.result.meshes) {
      for (let i = 0; i < m.vertices.length; i += 6) {
        const x = m.vertices[i]!
        const y = m.vertices[i + 1]!
        const z = m.vertices[i + 2]!
        const onCornerFilletWall =
          p.corner_radius > p.wall &&
          ((x <= p.corner_radius &&
            y <= p.corner_radius &&
            Math.hypot(x - p.corner_radius, y - p.corner_radius) >= innerR - 0.05) ||
            (x >= p.width - p.corner_radius &&
              y <= p.corner_radius &&
              Math.hypot(x - (p.width - p.corner_radius), y - p.corner_radius) >= innerR - 0.05) ||
            (x <= p.corner_radius &&
              y >= p.depth - p.corner_radius &&
              Math.hypot(x - p.corner_radius, y - (p.depth - p.corner_radius)) >= innerR - 0.05) ||
            (x >= p.width - p.corner_radius &&
              y >= p.depth - p.corner_radius &&
              Math.hypot(x - (p.width - p.corner_radius), y - (p.depth - p.corner_radius)) >= innerR - 0.05))
        if (
          z > p.floor + 0.5 &&
          z < p.height - 0.1 &&
          y > p.skadis_back_thickness + 0.05 &&
          y < p.depth - p.wall - 0.05 &&
          x > p.wall + 0.05 &&
          x < p.width - p.wall - 0.05 &&
          !onCornerFilletWall
        ) {
          cavityIntruder++
        }
        if (
          z > p.floor + 0.5 &&
          z <= p.height + 0.05 &&
          x >= 31 &&
          x <= 93 &&
          y > p.wall + 0.05 &&
          y < p.depth - p.wall - 0.05
        ) {
          thickBayIntruder++
        }
        if (Math.abs(z - p.floor) < 0.05 && x > 32 && x < 88 && y > p.wall + 0.2 && y < p.depth - p.wall - 0.2) {
          minFloorHoleY = Math.min(minFloorHoleY, y)
          maxFloorHoleY = Math.max(maxFloorHoleY, y)
        }
        if (
          z >= p.floor &&
          z <= p.floor + p.rib / 2 + 1.0 &&
          x > p.wall + 8 &&
          x < p.width - p.wall - 8 &&
          y > p.depth - 0.2
        ) {
          hasFlushBottomWallOpening = true
        }
      }
    }
    expect(cavityIntruder).toBe(0)
    expect(thickBayIntruder).toBe(0)
    expect(minFloorHoleY).toBeLessThanOrEqual(p.wall + p.floor_rib / 2 + 0.6)
    expect(maxFloorHoleY).toBeGreaterThanOrEqual(p.depth - p.wall - p.floor_rib / 2 - 0.5)
    expect(hasFlushBottomWallOpening).toBe(true)

    // Large-radius organic oval rounding without overhang roofs builds a closed manifold
    const oval = await engine.buildSource(
      generateSkadisVoronoiBox({ ...p, print_optimized: false, cell_radius: 6, floor_cell_radius: 6 }),
      request,
    )
    expectClosedManifold(oval.result.meshes)
    expect(oval.result.volume).toBeGreaterThan(0)
    expect(oval.result.volume).toBeLessThan(rounded.result.volume + 15000)

    // 4. Dovetail strips thicken locally to 7 mm and slots open all the way through z = 0
    const hookXs = [20, 100]
    for (const hx of hookXs) {
      let hasBottomSlotVertex = false
      let maxStripY = 0
      let plugTris = 0
      for (const m of rounded.result.meshes) {
        const v = m.vertices
        for (let i = 0; i < v.length; i += 6) {
          const x = v[i]!
          const y = v[i + 1]!
          const z = v[i + 2]!
          if (x >= hx - 4 && x <= hx + 8) {
            if (z > p.floor + 0.5 && z <= p.height + 0.05 && y <= p.skadis_back_thickness + 0.1) {
              maxStripY = Math.max(maxStripY, y)
            }
            if (Math.abs(z) < 0.05 && y > 1.0 && y < 4.5) {
              hasBottomSlotVertex = true
            }
          }
        }
        for (let i = 0; i < m.indices.length; i += 3) {
          const a = m.indices[i]! * 6
          const b = m.indices[i + 1]! * 6
          const c = m.indices[i + 2]! * 6
          const cx = (v[a]! + v[b]! + v[c]!) / 3
          const cy = (v[a + 1]! + v[b + 1]! + v[c + 1]!) / 3
          const cz = (v[a + 2]! + v[b + 2]! + v[c + 2]!) / 3
          if (cx >= hx - 2.0 && cx <= hx + 2.0 && cy >= 0.2 && cy <= 3.8 && cz >= 0.1 && cz <= 40) {
            plugTris++
          }
        }
      }
      expect(maxStripY).toBeGreaterThanOrEqual(p.skadis_back_thickness - 0.05)
      expect(hasBottomSlotVertex).toBe(true)
      expect(plugTris).toBe(0)
    }

    // 5. Outer box corner arcs are smoothly tessellated ($fn = 64 -> >= 12 interior arc vertices per 90 deg)
    const cornerArcVerts = new Set<string>()
    for (const m of rounded.result.meshes) {
      for (let i = 0; i < m.vertices.length; i += 6) {
        const x = m.vertices[i]!
        const y = m.vertices[i + 1]!
        const z = m.vertices[i + 2]!
        if (
          Math.abs(z - p.height) < 0.05 &&
          x > 0.05 &&
          x < p.corner_radius - 0.05 &&
          y > p.depth - p.corner_radius + 0.05 &&
          y < p.depth - 0.05
        ) {
          cornerArcVerts.add(`${x.toFixed(2)},${y.toFixed(2)}`)
        }
      }
    }
    // Default trabecular (legacy box-style organic lattice) with rounded oval cells builds cleanly
    const defaultOrganic = await engine.buildSource(
      generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, skadis_part: 1 }),
      request,
    )
    expectClosedManifold(defaultOrganic.result.meshes)
    expect(defaultOrganic.result.volume).toBeGreaterThan(0)
    expect(cornerArcVerts.size).toBeGreaterThanOrEqual(12)
  }, 120000)

  it('eliminates side-wall mirror cusps and rim slivers with continuous periodic trabecular wrap', () => {
    const baseOpts = {
      min: [0, 0] as const,
      max: [65, 73] as const,
      cell: 12,
      rib: 2.4,
      seed: 1,
      jitter: 0.7,
      overhang: 0,
      gradient: 1,
      gradientMode: 'vertical' as const,
      slope: 0,
      frame: 0,
      radius: 2,
      keepOut: [] as const,
      seamOffset: 280,
      seamDir: -1,
      perimeter: 364,
      openLeft: true,
      openRight: true,
    }
    const sidePolys = generateTrabecularCells(baseOpts)
    expect(sidePolys.length).toBeGreaterThanOrEqual(6)
    for (const poly of sidePolys) {
      expect(poly.length).toBeGreaterThanOrEqual(3)
      for (let i = 0; i < poly.length; i++) {
        const a = poly[i]!
        const b = poly[(i + 1) % poly.length]!
        const c = poly[(i + 2) % poly.length]!
        const cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])
        expect(cross).toBeGreaterThanOrEqual(-1e-5)
      }
    }

    const closedPolys = generateTrabecularCells({
      ...baseOpts,
      seamOffset: 0,
      seamDir: 1,
      perimeter: 0,
      openLeft: false,
      openRight: false,
    })
    expect(closedPolys.length).toBeGreaterThan(0)
    for (const poly of closedPolys) {
      const bb = bboxOfPoly(poly)
      expect(bb.height).toBeGreaterThanOrEqual(baseOpts.rib * 1.25)
      expect(bb.width).toBeGreaterThanOrEqual(baseOpts.rib * 1.25)
    }

    const seamStart = generateTrabecularCells({
      ...baseOpts,
      min: [0, 0],
      max: [10, 73],
      seamOffset: -5,
      seamDir: 1,
      perimeter: 300,
    })
    const seamWrap = generateTrabecularCells({
      ...baseOpts,
      min: [0, 0],
      max: [10, 73],
      seamOffset: 295,
      seamDir: 1,
      perimeter: 300,
    })
    expect(seamStart.length).toBe(seamWrap.length)
    for (let i = 0; i < seamStart.length; i++) {
      expect(seamStart[i]!.length).toBe(seamWrap[i]!.length)
      for (let j = 0; j < seamStart[i]!.length; j++) {
        expect(seamStart[i]![j]![0]).toBeCloseTo(seamWrap[i]![j]![0], 10)
        expect(seamStart[i]![j]![1]).toBeCloseTo(seamWrap[i]![j]![1], 10)
      }
    }
  })

  it('wraps trabecular and voronoi patterns continuously across rounded and square corners', async () => {
    const countMidCornerArcVerts = (
      meshes: Awaited<ReturnType<typeof engine.buildSource>>['result']['meshes'],
      p: SkadisVoronoiBoxParams,
    ) => {
      const verts = new Set<string>()
      for (const m of meshes) {
        for (let i = 0; i < m.vertices.length; i += 6) {
          const x = m.vertices[i]!
          const y = m.vertices[i + 1]!
          const z = m.vertices[i + 2]!
          if (z <= p.floor + 6 || z >= p.height - p.wall - 2) continue
          const inFrontLeft =
            x > 0.05 && x < p.corner_radius - 0.05 && y > p.depth - p.corner_radius + 0.05 && y < p.depth - 0.05
          const inFrontRight =
            x > p.width - p.corner_radius + 0.05 &&
            x < p.width - 0.05 &&
            y > p.depth - p.corner_radius + 0.05 &&
            y < p.depth - 0.05
          const inBackRight =
            x > p.width - p.corner_radius + 0.05 && x < p.width - 0.05 && y > 0.05 && y < p.corner_radius - 0.05
          const inBackLeft = x > 0.05 && x < p.corner_radius - 0.05 && y > 0.05 && y < p.corner_radius - 0.05
          if (inFrontLeft || inFrontRight || inBackRight || inBackLeft) {
            verts.add(`${x.toFixed(2)},${y.toFixed(2)},${z.toFixed(2)}`)
          }
        }
      }
      return verts.size
    }

    const roundedTrabecular = {
      ...SKADIS_VORONOI_BOX_DEFAULTS,
      corner_radius: 5,
      print_optimized: false,
      skadis_part: 1,
    }
    const trabRes = await engine.buildSource(generateSkadisVoronoiBox(roundedTrabecular), request)
    expectClosedManifold(trabRes.result.meshes)
    expect(countMidCornerArcVerts(trabRes.result.meshes, roundedTrabecular)).toBeGreaterThanOrEqual(6)

    const roundedVoronoiPrint = {
      ...SKADIS_VORONOI_BOX_DEFAULTS,
      lattice_mode: 'voronoi' as const,
      corner_radius: 5,
      print_optimized: true,
      skadis_part: 1,
    }
    const vorRes = await engine.buildSource(generateSkadisVoronoiBox(roundedVoronoiPrint), request)
    expectClosedManifold(vorRes.result.meshes)
    expect(countMidCornerArcVerts(vorRes.result.meshes, roundedVoronoiPrint)).toBeGreaterThanOrEqual(6)
    const audit = auditOverhangs(vorRes.result.meshes, 45, (cx, cy, cz) => {
      const inThroatZ = cz > 64 && cz < 68
      const inThroatY = cy >= -0.1 && cy <= 3.5
      const nearHookX = Math.abs(cx - 20) <= 3 || Math.abs(cx - 100) <= 3
      return inThroatZ && inThroatY && nearHookX
    })
    expect(audit.excessArea).toBeLessThan(0.02)

    const squareParams = { ...SKADIS_VORONOI_BOX_DEFAULTS, corner_radius: 0, print_optimized: false, skadis_part: 1 }
    const squareRes = await engine.buildSource(generateSkadisVoronoiBox(squareParams), request)
    expectClosedManifold(squareRes.result.meshes)
    let midCornerSeamVerts = 0
    for (const m of squareRes.result.meshes) {
      for (let i = 0; i < m.vertices.length; i += 6) {
        const x = m.vertices[i]!
        const y = m.vertices[i + 1]!
        const z = m.vertices[i + 2]!
        if (z <= squareParams.floor + 6 || z >= squareParams.height - squareParams.wall - 2) continue
        const onCornerEdge =
          ((Math.abs(x - squareParams.wall) < 0.05 || Math.abs(x - (squareParams.width - squareParams.wall)) < 0.05) &&
            y > squareParams.depth - squareParams.wall + 0.05) ||
          ((Math.abs(y - squareParams.wall) < 0.05 || Math.abs(y - (squareParams.depth - squareParams.wall)) < 0.05) &&
            x < squareParams.wall - 0.05)
        if (onCornerEdge) midCornerSeamVerts++
      }
    }
    expect(midCornerSeamVerts).toBeGreaterThan(0)
  }, 120000)

  it('keeps floor ribs unbroken with no self-intersecting horns around SKADIS dovetail collars', async () => {
    const p = { ...SKADIS_VORONOI_BOX_DEFAULTS, skadis_part: 1 }
    const { result } = await engine.buildSource(generateSkadisVoronoiBox(p), request)
    expectClosedManifold(result.meshes)

    const floorMesh = result.meshes.find(m => {
      let maxZ = -Infinity
      for (let i = 0; i < m.vertices.length; i += 6) maxZ = Math.max(maxZ, m.vertices[i + 2]!)
      return maxZ <= p.floor + 0.05
    })
    expect(floorMesh).toBeDefined()

    const v = floorMesh!.vertices
    const idx = floorMesh!.indices
    const coords = new Map<string, [number, number]>()
    const directed = new Map<string, { from: string; to: string }>()
    const ptKey = (x: number, y: number) => `${x.toFixed(3)},${y.toFixed(3)}`
    for (let i = 0; i < idx.length; i += 3) {
      const a = idx[i]! * 6
      const b = idx[i + 1]! * 6
      const c = idx[i + 2]! * 6
      if (Math.max(Math.abs(v[a + 2]! - p.floor), Math.abs(v[b + 2]! - p.floor), Math.abs(v[c + 2]! - p.floor)) > 0.02)
        continue
      const crossZ = (v[b]! - v[a]!) * (v[c + 1]! - v[a + 1]!) - (v[b + 1]! - v[a + 1]!) * (v[c]! - v[a]!)
      if (crossZ <= 1e-6) continue
      const ks = [ptKey(v[a]!, v[a + 1]!), ptKey(v[b]!, v[b + 1]!), ptKey(v[c]!, v[c + 1]!)] as const
      coords.set(ks[0], [v[a]!, v[a + 1]!])
      coords.set(ks[1], [v[b]!, v[b + 1]!])
      coords.set(ks[2], [v[c]!, v[c + 1]!])
      for (let e = 0; e < 3; e++) {
        const from = ks[e]!
        const to = ks[(e + 1) % 3]!
        if (from === to) continue
        const revKey = `${to}->${from}`
        if (directed.has(revKey)) directed.delete(revKey)
        else directed.set(`${from}->${to}`, { from, to })
      }
    }

    const adj = new Map<string, string[]>()
    for (const { from, to } of directed.values()) {
      const list = adj.get(from) ?? []
      list.push(to)
      adj.set(from, list)
    }
    const loops: Array<Array<[number, number]>> = []
    while (adj.size > 0) {
      const start = adj.keys().next().value!
      const loop: Array<[number, number]> = []
      let curr = start
      for (let guard = 0; guard < 2000; guard++) {
        loop.push(coords.get(curr)!)
        const nexts = adj.get(curr)
        if (!nexts || nexts.length === 0) {
          adj.delete(curr)
          break
        }
        const next = nexts.pop()!
        if (nexts.length === 0) adj.delete(curr)
        curr = next
        if (curr === start) break
      }
      if (loop.length >= 3) loops.push(loop)
    }

    const holeLoops = loops.filter(poly => {
      let area2 = 0
      for (let i = 0; i < poly.length; i++) {
        const a = poly[i]!
        const b = poly[(i + 1) % poly.length]!
        area2 += a[0] * b[1] - a[1] * b[0]
      }
      return area2 < -1e-2
    })
    expect(holeLoops.length).toBeGreaterThanOrEqual(8)

    const minKos = Math.cos((40 * Math.PI) / 180)
    const outerAndHoleLoops = loops.filter(poly => poly.length >= 3)
    expect(outerAndHoleLoops.length).toBeGreaterThan(holeLoops.length)
    for (const loop of holeLoops) {
      const bb = bboxOfPoly(loop)
      expect(Math.max(bb.width, bb.height)).toBeLessThanOrEqual(p.floor_cell_size * 2.3)
    }
    for (const loop of outerAndHoleLoops) {
      for (let i = 0; i < loop.length; i++) {
        const prev = loop[(i + loop.length - 1) % loop.length]!
        const curr = loop[i]!
        const next = loop[(i + 1) % loop.length]!
        const ux = prev[0] - curr[0]
        const uy = prev[1] - curr[1]
        const vx = next[0] - curr[0]
        const vy = next[1] - curr[1]
        const lu = Math.hypot(ux, uy)
        const lv = Math.hypot(vx, vy)
        if (lu < 0.25 || lv < 0.25) continue
        const cosVertex = (ux * vx + uy * vy) / (lu * lv)
        expect(cosVertex).toBeLessThan(minKos)
      }
    }
  }, 120000)

  it('transitions floor lattice continuously into all 4 walls when wall and floor parameters match', async () => {
    const pointInTris2D = (
      tris: Array<readonly [number, number, number, number, number, number]>,
      px: number,
      py: number,
    ) => {
      for (const [ax, ay, bx, by, cx, cy] of tris) {
        const d1 = (bx - ax) * (py - ay) - (by - ay) * (px - ax)
        const d2 = (cx - bx) * (py - by) - (cy - by) * (px - bx)
        const d3 = (ax - cx) * (py - cy) - (ay - cy) * (px - cx)
        if (d1 >= -1e-5 && d2 >= -1e-5 && d3 >= -1e-5) return true
        if (d1 <= 1e-5 && d2 <= 1e-5 && d3 <= 1e-5) return true
      }
      return false
    }

    const extractProjectedTris = (
      meshes: Awaited<ReturnType<typeof engine.buildSource>>['result']['meshes'],
      p: SkadisVoronoiBoxParams,
    ) => {
      const floorTris: Array<readonly [number, number, number, number, number, number]> = []
      const frontTris: Array<readonly [number, number, number, number, number, number]> = []
      const rightTris: Array<readonly [number, number, number, number, number, number]> = []
      for (const m of meshes) {
        const v = m.vertices
        for (let i = 0; i < m.indices.length; i += 3) {
          const a = m.indices[i]! * 6
          const b = m.indices[i + 1]! * 6
          const c = m.indices[i + 2]! * 6
          if (
            Math.max(Math.abs(v[a + 2]! - p.floor), Math.abs(v[b + 2]! - p.floor), Math.abs(v[c + 2]! - p.floor)) < 0.02
          ) {
            const cz = (v[b]! - v[a]!) * (v[c + 1]! - v[a + 1]!) - (v[b + 1]! - v[a + 1]!) * (v[c]! - v[a]!)
            if (cz > 1e-6) floorTris.push([v[a]!, v[a + 1]!, v[b]!, v[b + 1]!, v[c]!, v[c + 1]!])
          }
          if (Math.min(v[a + 1]!, v[b + 1]!, v[c + 1]!) > p.depth - 0.05) {
            frontTris.push([v[a]!, v[a + 2]!, v[b]!, v[b + 2]!, v[c]!, v[c + 2]!])
          }
          if (Math.min(v[a]!, v[b]!, v[c]!) > p.width - 0.05) {
            rightTris.push([v[a + 1]!, v[a + 2]!, v[b + 1]!, v[b + 2]!, v[c + 1]!, v[c + 2]!])
          }
        }
      }
      return { floorTris, frontTris, rightTris }
    }

    const countBottomGussetVerts = (
      meshes: Awaited<ReturnType<typeof engine.buildSource>>['result']['meshes'],
      p: SkadisVoronoiBoxParams,
    ) => {
      let count = 0
      for (const m of meshes) {
        for (let i = 0; i < m.vertices.length; i += 6) {
          const x = m.vertices[i]!
          const y = m.vertices[i + 1]!
          const z = m.vertices[i + 2]!
          if (z <= p.floor + 0.35 || z >= p.floor + 2.6) continue
          const inFrontBand =
            x > p.corner_radius + 4 &&
            x < p.width - p.corner_radius - 4 &&
            y > p.depth - p.wall - 2.6 &&
            y < p.depth - p.wall - 0.25
          const inBackBayBand = x > 33 && x < 91 && y > p.wall + 0.25 && y < p.wall + 2.6
          if (inFrontBand || inBackBayBand) count++
        }
      }
      return count
    }

    for (const variant of [
      { ...SKADIS_VORONOI_BOX_DEFAULTS, skadis_part: 1 },
      {
        ...SKADIS_VORONOI_BOX_DEFAULTS,
        lattice_mode: 'voronoi' as const,
        floor_lattice_mode: 'voronoi' as const,
        cell_size: 14,
        floor_cell_size: 14,
        skadis_part: 1,
      },
    ]) {
      const { result } = await engine.buildSource(generateSkadisVoronoiBox(variant), request)
      expectClosedManifold(result.meshes)
      expect(countBottomGussetVerts(result.meshes, variant)).toBeGreaterThanOrEqual(24)
      const { floorTris, frontTris, rightTris } = extractProjectedTris(result.meshes, variant)

      let frontMatch = 0
      let frontOpenFloor = 0
      const samplesX = 48
      const wallHits: boolean[] = []
      const floorHits: boolean[] = []
      for (let k = 0; k < samplesX; k++) {
        const x = 12 + ((variant.width - 24) * k) / (samplesX - 1)
        const wallSolid = pointInTris2D(frontTris, x, variant.floor + 0.3)
        const floorSolid = pointInTris2D(floorTris, x, variant.depth - variant.wall - 0.5)
        wallHits.push(wallSolid)
        floorHits.push(floorSolid)
        if (!floorSolid) frontOpenFloor++
        if (wallSolid === floorSolid) frontMatch++
      }
      expect(frontOpenFloor).toBeGreaterThan(6)
      expect(frontMatch / samplesX).toBeGreaterThan(0.88)
      // Every contiguous wall rib along the front edge connects into a floor rib within <= 1 sample step (2 mm).
      for (let k = 0; k < samplesX; k++) {
        if (wallHits[k] && (k === 0 || !wallHits[k - 1])) {
          let end = k
          while (end + 1 < samplesX && wallHits[end + 1]) end++
          expect(floorHits.slice(Math.max(0, k - 1), end + 2).some(Boolean)).toBe(true)
        }
      }

      let rightMatch = 0
      let rightOpenFloor = 0
      const samplesY = 32
      for (let k = 0; k < samplesY; k++) {
        const y = 12 + ((variant.depth - 24) * k) / (samplesY - 1)
        const wallSolid = pointInTris2D(rightTris, y, variant.floor + 0.3)
        const floorSolid = pointInTris2D(floorTris, variant.width - variant.wall - 0.5, y)
        if (!floorSolid) rightOpenFloor++
        if (wallSolid === floorSolid) rightMatch++
      }
      expect(rightOpenFloor).toBeGreaterThan(4)
      expect(rightMatch / samplesY).toBeGreaterThan(0.85)

      // Zero-step seam across inner_cavity_2d() at z = floor * 0.5 (straight front edge + Front-Left rounded corner).
      const zMidFloor = variant.floor * 0.5
      const inSolidXZ = (px: number, py: number) => {
        for (const m of result.meshes) {
          const v = m.vertices
          const idx = m.indices
          let crossings = 0
          for (let i = 0; i < idx.length; i += 3) {
            const a = idx[i]! * 6
            const b = idx[i + 1]! * 6
            const c = idx[i + 2]! * 6
            const x1 = v[a]!
            const y1 = v[a + 1]!
            const x2 = v[b]!
            const y2 = v[b + 1]!
            const x3 = v[c]!
            const y3 = v[c + 1]!
            if (
              Math.min(x1, x2, x3) > px ||
              Math.max(x1, x2, x3) < px ||
              Math.min(y1, y2, y3) > py ||
              Math.max(y1, y2, y3) < py
            )
              continue
            const d1 = (px - x2) * (y1 - y2) - (x1 - x2) * (py - y2)
            const d2 = (px - x3) * (y2 - y3) - (x2 - x3) * (py - y3)
            const d3 = (px - x1) * (y3 - y1) - (x3 - x1) * (py - y1)
            if (!((d1 < 0 || d2 < 0 || d3 < 0) && (d1 > 0 || d2 > 0 || d3 > 0))) {
              const det = (y2 - y3) * (x1 - x3) + (x3 - x2) * (y1 - y3)
              if (Math.abs(det) < 1e-12) continue
              const l1 = ((y2 - y3) * (px - x3) + (x3 - x2) * (py - y3)) / det
              const l2 = ((y3 - y1) * (px - x3) + (x1 - x3) * (py - y3)) / det
              const z = l1 * v[a + 2]! + l2 * v[b + 2]! + (1 - l1 - l2) * v[c + 2]!
              if (z < zMidFloor) crossings++
            }
          }
          if (crossings % 2 === 1) return true
        }
        return false
      }
      let seamMatch = 0
      const seamSamples = 40
      for (let k = 0; k < seamSamples; k++) {
        const x = 10 + ((variant.width - 20) * k) / (seamSamples - 1) + 0.0013
        const floorHit = inSolidXZ(x, variant.depth - variant.wall - 0.15)
        const wallHit = inSolidXZ(x, variant.depth - variant.wall + 0.15)
        if (floorHit === wallHit) seamMatch++
      }
      expect(seamMatch / seamSamples).toBeGreaterThanOrEqual(0.95)

      let cornerSeamMatch = 0
      const cornerSamples = 20
      const innerR = variant.corner_radius - variant.wall
      const flCx = variant.corner_radius
      const flCy = variant.depth - variant.corner_radius
      for (let k = 0; k < cornerSamples; k++) {
        const ang = Math.PI * 0.5 + (Math.PI * 0.5 * (k + 0.5)) / cornerSamples
        const ux = Math.cos(ang)
        const uy = Math.sin(ang)
        const floorHit = inSolidXZ(flCx + (innerR - 0.15) * ux, flCy + (innerR - 0.15) * uy)
        const wallHit = inSolidXZ(flCx + (innerR + 0.15) * ux, flCy + (innerR + 0.15) * uy)
        if (floorHit === wallHit) cornerSeamMatch++
      }
      expect(cornerSeamMatch / cornerSamples).toBeGreaterThanOrEqual(0.95)
    }

    const uncoupled = { ...SKADIS_VORONOI_BOX_DEFAULTS, floor_cell_size: 19, skadis_part: 1 }
    const uncoupledRes = await engine.buildSource(generateSkadisVoronoiBox(uncoupled), request)
    expect(countBottomGussetVerts(uncoupledRes.result.meshes, uncoupled)).toBe(0)
    const { floorTris: uncoupledFloor } = extractProjectedTris(uncoupledRes.result.meshes, uncoupled)
    let uncoupledOpenFloor = 0
    for (let k = 0; k < 48; k++) {
      const x = 12 + ((uncoupled.width - 24) * k) / 47
      if (!pointInTris2D(uncoupledFloor, x, uncoupled.depth - uncoupled.wall - 0.8)) uncoupledOpenFloor++
    }
    expect(uncoupledOpenFloor).toBe(0)
  }, 120000)

  it('produces uniform interior coupled floor cells without oversized voids or 4-way X-junctions on wide 240 mm boxes', async () => {
    const width = 240
    const depth = 75
    const wall = 2
    const cornerR = 5
    const innerR = cornerR - wall
    const spanX = width - 2 * cornerR
    const sideSpan = depth - 2 * cornerR
    const cornerArc = cornerR * Math.PI * 0.5
    const perimeter = 2 * spanX + 2 * sideSpan + 4 * cornerArc
    const coupledCells = emitCoupledVoronoiFloor(
      {
        min: [wall, wall],
        max: [width - wall, depth - wall],
        cell: 12,
        rib: 2.4,
        seed: 72,
        jitter: 0.7,
        gradient: 1,
        radius: 2,
        keepOut: [],
      },
      {
        perimeter,
        spanX,
        sideSpan,
        cornerArc,
        innerR,
        sShift: 30,
        wallCell: 12,
        wallRib: 2.4,
        wallSeed: 1,
        wallJitter: 0.7,
        wallOverhang: 0,
        wallHeight: 76,
      },
    )
    expect(coupledCells.length).toBeGreaterThanOrEqual(70)
    for (const poly of coupledCells) {
      const bb = bboxOfPoly(poly)
      expect(bb.width).toBeLessThanOrEqual(23.5)
      expect(bb.height).toBeLessThanOrEqual(23.5)
      expect(Math.abs(signedArea(poly))).toBeLessThanOrEqual(300)
    }

    const wide = {
      ...SKADIS_VORONOI_BOX_DEFAULTS,
      width,
      floor_frame: false,
      edge_frame: false,
      skadis_part: 1,
    }
    const { result } = await engine.buildSource(generateSkadisVoronoiBox(wide), request)
    expectClosedManifold(result.meshes)
  }, 120000)

  it('keeps back wall bays, SKADIS dovetail columns, and dividers intact when Customizer variables (width, hook_count, dividers) are edited directly in SCAD', async () => {
    const baseScad = generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, skadis_part: 1 })
    const hasColumnAt = (meshes: Awaited<ReturnType<typeof engine.buildSource>>['result']['meshes'], xTarget: number) =>
      meshes.some(m => {
        if (m.vertices.length === 0) return false
        let minX = Infinity
        let maxX = -Infinity
        let minY = Infinity
        let maxY = -Infinity
        let maxZ = -Infinity
        for (let i = 0; i < m.vertices.length; i += 6) {
          const x = m.vertices[i]!
          const y = m.vertices[i + 1]!
          const z = m.vertices[i + 2]!
          if (x < minX) minX = x
          if (x > maxX) maxX = x
          if (y < minY) minY = y
          if (y > maxY) maxY = y
          if (z > maxZ) maxZ = z
        }
        return (
          minX <= xTarget - 4 &&
          maxX >= xTarget + 4 &&
          minX >= xTarget - 12 &&
          maxX <= xTarget + 12 &&
          minY <= 0.1 &&
          maxY >= 6.9 &&
          maxY <= 7.2 &&
          maxZ >= 79.9
        )
      })
    const maxBackBayX = (meshes: Awaited<ReturnType<typeof engine.buildSource>>['result']['meshes']) => {
      let maxX = -Infinity
      for (const m of meshes) {
        for (let i = 0; i < m.vertices.length; i += 6) {
          const x = m.vertices[i]!
          const y = m.vertices[i + 1]!
          const z = m.vertices[i + 2]!
          if (y <= 2.1 && z > 20 && z < 70) maxX = Math.max(maxX, x)
        }
      }
      return maxX
    }

    const widened = baseScad.replace('width = 120;', 'width = 180;').replace('dividers = 0;', 'dividers = 1;')
    const widenedRes = await engine.buildSource(widened, request)
    expectClosedManifold(widenedRes.result.meshes)
    expect(maxBackBayX(widenedRes.result.meshes)).toBeGreaterThan(170)
    expect(hasColumnAt(widenedRes.result.meshes, 50)).toBe(true)
    expect(hasColumnAt(widenedRes.result.meshes, 130)).toBe(true)
    expect(hasColumnAt(widenedRes.result.meshes, 20)).toBe(false)
    let hasCenterDivider = false
    for (const m of widenedRes.result.meshes) {
      for (let i = 0; i < m.vertices.length; i += 6) {
        const x = m.vertices[i]!
        const y = m.vertices[i + 1]!
        const z = m.vertices[i + 2]!
        if (Math.abs(x - 90) <= 1.2 && y > 10 && y < 65 && z > 20 && z < 70) {
          hasCenterDivider = true
          break
        }
      }
    }
    expect(hasCenterDivider).toBe(true)

    // Default hook_count = 0 auto-scales hook columns with box width: 210 mm -> 3 hooks (25, 105, 185), 280 mm -> 4 hooks (20, 100, 180, 260).
    const autoThreeHooks = baseScad.replace('width = 120;', 'width = 210;')
    const autoThreeRes = await engine.buildSource(autoThreeHooks, request)
    expectClosedManifold(autoThreeRes.result.meshes)
    expect(maxBackBayX(autoThreeRes.result.meshes)).toBeGreaterThan(200)
    for (const hx of [25, 105, 185]) expect(hasColumnAt(autoThreeRes.result.meshes, hx)).toBe(true)

    const autoFourHooks = baseScad.replace('width = 120;', 'width = 280;')
    const autoFourRes = await engine.buildSource(autoFourHooks, request)
    expectClosedManifold(autoFourRes.result.meshes)
    expect(maxBackBayX(autoFourRes.result.meshes)).toBeGreaterThan(270)
    for (const hx of [20, 100, 180, 260]) expect(hasColumnAt(autoFourRes.result.meshes, hx)).toBe(true)

    // Manual override on a wide box: hook_count = 2 keeps 2 columns (80, 160), while hook_count = 6 at 40 mm pitch fits 6 columns.
    const manualTwo = baseScad.replace('width = 120;', 'width = 240;').replace('hook_count = 0;', 'hook_count = 2;')
    const manualTwoRes = await engine.buildSource(manualTwo, request)
    expectClosedManifold(manualTwoRes.result.meshes)
    expect(hasColumnAt(manualTwoRes.result.meshes, 80)).toBe(true)
    expect(hasColumnAt(manualTwoRes.result.meshes, 160)).toBe(true)
    expect(hasColumnAt(manualTwoRes.result.meshes, 40)).toBe(false)

    const manualSix = baseScad
      .replace('width = 120;', 'width = 240;')
      .replace('skadis_spacing_steps = 2;', 'skadis_spacing_steps = 1;')
      .replace('hook_count = 0;', 'hook_count = 6;')
    const manualSixRes = await engine.buildSource(manualSix, request)
    expectClosedManifold(manualSixRes.result.meshes)
    for (const hx of [20, 60, 100, 140, 180, 220]) expect(hasColumnAt(manualSixRes.result.meshes, hx)).toBe(true)
  }, 120000)

  it('supports independent bottom_fillet, edge_frame, and floor_frame options in generator params and live Customizer SCAD edits', async () => {
    const base = { ...SKADIS_VORONOI_BOX_DEFAULTS, skadis_part: 1 }
    const defaultScad = generateSkadisVoronoiBox(base)
    const customParams = extractCustomizerParameters(defaultScad)
    expect(customParams.find(p => p.name === 'bottom_fillet')?.value).toBe(true)
    expect(customParams.find(p => p.name === 'edge_frame')?.value).toBe(false)
    expect(customParams.find(p => p.name === 'floor_frame')?.value).toBe(false)

    const pointInTris2D = (
      tris: ReadonlyArray<readonly [number, number, number, number, number, number]>,
      px: number,
      py: number,
    ) => {
      for (const [ax, ay, bx, by, cx, cy] of tris) {
        const d1 = (px - bx) * (ay - by) - (ax - bx) * (py - by)
        const d2 = (px - cx) * (by - cy) - (bx - cx) * (py - cy)
        const d3 = (px - ax) * (cy - ay) - (cx - ax) * (py - ay)
        if (!((d1 < -1e-4 || d2 < -1e-4 || d3 < -1e-4) && (d1 > 1e-4 || d2 > 1e-4 || d3 > 1e-4))) return true
      }
      return false
    }

    const inspectVariant = (
      meshes: Awaited<ReturnType<typeof engine.buildSource>>['result']['meshes'],
      p: SkadisVoronoiBoxParams,
    ) => {
      const floorTris: Array<readonly [number, number, number, number, number, number]> = []
      const frontTris: Array<readonly [number, number, number, number, number, number]> = []
      let wallGussetVerts = 0
      let cornerGussetVerts = 0
      const cxEff = Math.max(p.wall, p.corner_radius)
      const frCx = p.width - p.corner_radius
      const frCy = p.depth - p.corner_radius

      for (const m of meshes) {
        const v = m.vertices
        for (let i = 0; i < v.length; i += 6) {
          const x = v[i]!
          const y = v[i + 1]!
          const z = v[i + 2]!
          if (z > p.floor + 0.1 && z < p.floor + 2.6) {
            if (
              x >= cxEff - 0.05 &&
              x <= p.width - cxEff + 0.05 &&
              y > p.depth - p.wall - 2.6 &&
              y < p.depth - p.wall - 0.1
            ) {
              wallGussetVerts++
            }
            if (p.corner_radius > p.wall + 0.5 && x > frCx + 0.2 && y > frCy + 0.2) {
              const r = Math.hypot(x - frCx, y - frCy)
              if (r >= 0.35 && r <= p.corner_radius - p.wall - 0.05) cornerGussetVerts++
            }
          }
        }
        for (let i = 0; i < m.indices.length; i += 3) {
          const a = m.indices[i]! * 6
          const b = m.indices[i + 1]! * 6
          const c = m.indices[i + 2]! * 6
          if (
            Math.max(Math.abs(v[a + 2]! - p.floor), Math.abs(v[b + 2]! - p.floor), Math.abs(v[c + 2]! - p.floor)) < 0.02
          ) {
            const cz = (v[b]! - v[a]!) * (v[c + 1]! - v[a + 1]!) - (v[b + 1]! - v[a + 1]!) * (v[c]! - v[a]!)
            if (cz > 1e-6) floorTris.push([v[a]!, v[a + 1]!, v[b]!, v[b + 1]!, v[c]!, v[c + 1]!])
          }
          if (Math.min(v[a + 1]!, v[b + 1]!, v[c + 1]!) > p.depth - 0.05) {
            frontTris.push([v[a]!, v[a + 2]!, v[b]!, v[b + 2]!, v[c]!, v[c + 2]!])
          }
        }
      }

      let frontRimOpen = 0
      let frontMidFloorOpen = 0
      let frontOuterFloorOpen = 0
      let frontBottomRailOpen = 0
      let frontFloorBandOpen = 0
      for (let k = 0; k < 48; k++) {
        const x = 12 + ((p.width - 24) * k) / 47
        if (!pointInTris2D(floorTris, x, p.depth - p.wall - 0.8)) frontRimOpen++
        if (!pointInTris2D(floorTris, x, p.depth - p.wall - 2.0)) frontMidFloorOpen++
        if (!pointInTris2D(floorTris, x, p.depth - p.wall - 3.6)) frontOuterFloorOpen++
        if (!pointInTris2D(frontTris, x, p.floor + 0.6)) frontBottomRailOpen++
        if (!pointInTris2D(frontTris, x, p.floor * 0.5)) frontFloorBandOpen++
      }
      const hasFrontGussetSpan = meshes.some(m => {
        if (m.vertices.length === 0) return false
        let minX = Infinity
        let maxX = -Infinity
        let minY = Infinity
        let maxY = -Infinity
        let maxZ = -Infinity
        for (let i = 0; i < m.vertices.length; i += 6) {
          minX = Math.min(minX, m.vertices[i]!)
          maxX = Math.max(maxX, m.vertices[i]!)
          minY = Math.min(minY, m.vertices[i + 1]!)
          maxY = Math.max(maxY, m.vertices[i + 1]!)
          maxZ = Math.max(maxZ, m.vertices[i + 2]!)
        }
        return (
          minX <= cxEff + 0.1 &&
          maxX >= p.width - cxEff - 0.1 &&
          minY >= p.depth - p.wall - 3 &&
          maxY <= p.depth - p.wall + 0.05 &&
          maxZ >= p.floor + 2.0 &&
          maxZ <= p.floor + 3.0
        )
      })
      const hasSolidCornerPost = meshes.some(m => {
        if (m.vertices.length === 0) return false
        let minX = Infinity
        let maxY = -Infinity
        let minZ = Infinity
        let maxZ = -Infinity
        let midZVerts = 0
        for (let i = 0; i < m.vertices.length; i += 6) {
          const x = m.vertices[i]!
          const y = m.vertices[i + 1]!
          const z = m.vertices[i + 2]!
          minX = Math.min(minX, x)
          maxY = Math.max(maxY, y)
          minZ = Math.min(minZ, z)
          maxZ = Math.max(maxZ, z)
          if (z > 5 && z < p.height - 5) midZVerts++
        }
        return (
          minX >= p.width - cxEff - 0.2 &&
          maxY >= p.depth - 0.1 &&
          minZ <= 0.1 &&
          maxZ >= p.height - 0.1 &&
          midZVerts === 0
        )
      })
      return {
        wallGussetVerts,
        cornerGussetVerts,
        frontRimOpen,
        frontMidFloorOpen,
        frontOuterFloorOpen,
        frontBottomRailOpen,
        frontFloorBandOpen,
        hasSolidCornerPost,
        hasFrontGussetSpan,
      }
    }

    const defaultRes = await engine.buildSource(defaultScad, request)
    expectClosedManifold(defaultRes.result.meshes)
    const sDefault = inspectVariant(defaultRes.result.meshes, base)
    expect(sDefault.wallGussetVerts).toBeGreaterThanOrEqual(6)
    expect(sDefault.frontRimOpen).toBeGreaterThan(6)
    expect(sDefault.frontFloorBandOpen).toBeGreaterThan(6)

    const wideOpenParams = { ...base, width: 240, hook_count: 0, edge_frame: false, floor_frame: false }
    const wideOpenRes = await engine.buildSource(generateSkadisVoronoiBox(wideOpenParams), request)
    expectClosedManifold(wideOpenRes.result.meshes)
    const sWideOpen = inspectVariant(wideOpenRes.result.meshes, wideOpenParams)
    expect(sWideOpen.frontFloorBandOpen).toBeGreaterThan(10)

    const wideFrameParams = { ...base, width: 240, hook_count: 0, edge_frame: false, floor_frame: true }
    const wideFrameRes = await engine.buildSource(generateSkadisVoronoiBox(wideFrameParams), request)
    expectClosedManifold(wideFrameRes.result.meshes)
    const sWideFrame = inspectVariant(wideFrameRes.result.meshes, wideFrameParams)
    expect(sWideFrame.frontFloorBandOpen).toBe(0)
    expect(sWideFrame.frontRimOpen).toBe(0)

    const noFilletNoFrame = { ...base, bottom_fillet: false, edge_frame: false, floor_frame: false }
    const noFilletRes = await engine.buildSource(generateSkadisVoronoiBox(noFilletNoFrame), request)
    expectClosedManifold(noFilletRes.result.meshes)
    const s00 = inspectVariant(noFilletRes.result.meshes, noFilletNoFrame)
    expect(s00.wallGussetVerts).toBe(0)
    expect(s00.cornerGussetVerts).toBe(0)
    expect(s00.hasFrontGussetSpan).toBe(false)
    expect(s00.frontRimOpen).toBeGreaterThan(6)
    expect(s00.frontFloorBandOpen).toBeGreaterThan(6)

    const floorFrameFillet = { ...base, bottom_fillet: true, edge_frame: false, floor_frame: true }
    const floorFrameFilletRes = await engine.buildSource(generateSkadisVoronoiBox(floorFrameFillet), request)
    expectClosedManifold(floorFrameFilletRes.result.meshes)
    const sFloorFillet = inspectVariant(floorFrameFilletRes.result.meshes, floorFrameFillet)
    expect(sFloorFillet.hasSolidCornerPost).toBe(false)
    expect(sFloorFillet.wallGussetVerts).toBeGreaterThanOrEqual(6)
    expect(sFloorFillet.frontRimOpen).toBe(0)
    expect(sFloorFillet.frontOuterFloorOpen).toBe(0)
    expect(sFloorFillet.frontFloorBandOpen).toBe(0)

    const floorFrameOnly = { ...base, bottom_fillet: false, edge_frame: false, floor_frame: true }
    const floorFrameOnlyRes = await engine.buildSource(generateSkadisVoronoiBox(floorFrameOnly), request)
    expectClosedManifold(floorFrameOnlyRes.result.meshes)
    const sFloorOnly = inspectVariant(floorFrameOnlyRes.result.meshes, floorFrameOnly)
    expect(sFloorOnly.hasSolidCornerPost).toBe(false)
    expect(sFloorOnly.wallGussetVerts).toBe(0)
    expect(sFloorOnly.frontRimOpen).toBe(0)
    expect(sFloorOnly.frontMidFloorOpen).toBe(0)
    expect(sFloorOnly.frontFloorBandOpen).toBe(0)

    const filletAndBothFrames = { ...base, bottom_fillet: true, edge_frame: true, floor_frame: true }
    const bothRes = await engine.buildSource(generateSkadisVoronoiBox(filletAndBothFrames), request)
    expectClosedManifold(bothRes.result.meshes)
    const s11 = inspectVariant(bothRes.result.meshes, filletAndBothFrames)
    expect(s11.wallGussetVerts).toBeGreaterThanOrEqual(6)
    expect(s11.hasFrontGussetSpan).toBe(true)
    expect(s11.cornerGussetVerts).toBeGreaterThanOrEqual(20)
    expect(s11.frontRimOpen).toBe(0)
    expect(s11.frontOuterFloorOpen).toBe(0)
    expect(s11.frontBottomRailOpen).toBe(0)
    expect(s11.frontFloorBandOpen).toBe(0)
    expect(s11.hasSolidCornerPost).toBe(true)

    const wallFrameAndFillet = { ...base, bottom_fillet: true, edge_frame: true, floor_frame: false }
    const wallOnlyFilletRes = await engine.buildSource(generateSkadisVoronoiBox(wallFrameAndFillet), request)
    expectClosedManifold(wallOnlyFilletRes.result.meshes)
    const sWallFillet = inspectVariant(wallOnlyFilletRes.result.meshes, wallFrameAndFillet)
    expect(sWallFillet.wallGussetVerts).toBeGreaterThanOrEqual(6)
    expect(sWallFillet.cornerGussetVerts).toBeGreaterThanOrEqual(20)
    expect(sWallFillet.frontRimOpen).toBe(0)
    expect(sWallFillet.frontOuterFloorOpen).toBeGreaterThan(4)
    expect(sWallFillet.frontBottomRailOpen).toBe(0)
    expect(sWallFillet.frontFloorBandOpen).toBe(0)
    expect(sWallFillet.hasSolidCornerPost).toBe(true)

    const frameOnly = { ...base, bottom_fillet: false, edge_frame: true, floor_frame: false }
    const frameOnlyRes = await engine.buildSource(generateSkadisVoronoiBox(frameOnly), request)
    expectClosedManifold(frameOnlyRes.result.meshes)
    const s01 = inspectVariant(frameOnlyRes.result.meshes, frameOnly)
    expect(s01.wallGussetVerts).toBe(0)
    expect(s01.cornerGussetVerts).toBe(0)
    expect(s01.hasFrontGussetSpan).toBe(false)
    expect(s01.frontOuterFloorOpen).toBeGreaterThan(4)
    expect(s01.frontBottomRailOpen).toBe(0)
    expect(s01.frontFloorBandOpen).toBe(0)
    expect(s01.hasSolidCornerPost).toBe(true)

    const squareFrame = { ...base, corner_radius: 0, bottom_fillet: true, edge_frame: true, floor_frame: true }
    const squareFrameRes = await engine.buildSource(generateSkadisVoronoiBox(squareFrame), request)
    expectClosedManifold(squareFrameRes.result.meshes)
    const sSquare = inspectVariant(squareFrameRes.result.meshes, squareFrame)
    expect(sSquare.wallGussetVerts).toBeGreaterThanOrEqual(6)
    expect(sSquare.hasFrontGussetSpan).toBe(true)
    expect(sSquare.frontRimOpen).toBe(0)
    expect(sSquare.frontOuterFloorOpen).toBe(0)
    expect(sSquare.frontBottomRailOpen).toBe(0)
    expect(sSquare.frontFloorBandOpen).toBe(0)

    const liveEdited = defaultScad
      .replace('edge_frame = false;', 'edge_frame = true;')
      .replace('floor_frame = false;', 'floor_frame = true;')
    const liveEditedRes = await engine.buildSource(liveEdited, request)
    expectClosedManifold(liveEditedRes.result.meshes)
    const sLive = inspectVariant(liveEditedRes.result.meshes, filletAndBothFrames)
    expect(sLive.wallGussetVerts).toBeGreaterThanOrEqual(6)
    expect(sLive.hasFrontGussetSpan).toBe(true)
    expect(sLive.cornerGussetVerts).toBeGreaterThanOrEqual(20)
    expect(sLive.frontRimOpen).toBe(0)
    expect(sLive.frontOuterFloorOpen).toBe(0)
    expect(sLive.frontBottomRailOpen).toBe(0)
    expect(sLive.frontFloorBandOpen).toBe(0)

    const liveNoFillet = liveEdited.replace('bottom_fillet = true;', 'bottom_fillet = false;')
    const liveNoFilletRes = await engine.buildSource(liveNoFillet, request)
    expectClosedManifold(liveNoFilletRes.result.meshes)
    const sLiveNoFillet = inspectVariant(liveNoFilletRes.result.meshes, { ...frameOnly, floor_frame: true })
    expect(sLiveNoFillet.wallGussetVerts).toBe(0)
    expect(sLiveNoFillet.hasFrontGussetSpan).toBe(false)
    expect(sLiveNoFillet.frontMidFloorOpen).toBe(0)
  }, 120000)

  it('emits localized inline comments on every top-level Customizer parameter and exposes descriptive labels', () => {
    for (const locale of ['ru', 'en'] as const) {
      for (const mount_system of ['skadis_dovetail', 'skadis_integral', 'rail_dovetail', 'none'] as const) {
        const scad = generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, mount_system }, locale)
        const params = extractCustomizerParameters(scad)
        expect(params.length).toBeGreaterThanOrEqual(24)
        for (const param of params) {
          const fallback = param.name.replace(/[_-]+/g, ' ').replace(/^./, c => c.toUpperCase())
          expect(param.label).not.toBe(fallback)
          const line = scad.split('\n').find(l => l.startsWith(`${param.name} = `))
          expect(line).toBeDefined()
          expect(line).toMatch(/;\s*\/\/\s+\S+/)
        }
        const filletParam = params.find(p => p.name === 'bottom_fillet')
        const frameParam = params.find(p => p.name === 'edge_frame')
        const floorFrameParam = params.find(p => p.name === 'floor_frame')
        expect(typeof filletParam?.value).toBe('boolean')
        expect(filletParam?.options).toBeUndefined()
        expect(typeof frameParam?.value).toBe('boolean')
        expect(frameParam?.options).toBeUndefined()
        expect(typeof floorFrameParam?.value).toBe('boolean')
        expect(floorFrameParam?.options).toBeUndefined()
        if (locale === 'ru') {
          expect(filletParam?.label).toMatch(/галтел|стык/i)
          expect(frameParam?.label).toMatch(/Каркас|стенок/i)
          expect(floorFrameParam?.label).toMatch(/Каркас.*дна|перемыч/i)
        } else {
          expect(filletParam?.label).toMatch(/fillet|gusset/i)
          expect(frameParam?.label).toMatch(/frame|wall/i)
          expect(floorFrameParam?.label).toMatch(/floor.*frame|border/i)
        }
      }
    }
  })

  it('rejects invalid SKADIS dimensions, curved SKADIS, and sub-nozzle ribs', () => {
    expect(() => generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, width: 90 })).toThrow(/узк|narrow/)
    expect(() => generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, skadis_back_thickness: 5 })).toThrow(/7/)
    expect(() => generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, bend_radius: 160 })).toThrow(/плоск|flat/)
    expect(() =>
      generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, rib: 1.0, line_width: 0.6, min_rib_lines: 2 }),
    ).toThrow(/сопла|lines/)
  })

  it('exports SKADIS + Voronoi box 3MF with skadis_body assembly, named sub-parts, independent hooks, and colors', async () => {
    const { inflateRawSync } = await import('node:zlib')
    const { exportMesh } = await import('../src/exchange/meshConvert')
    const { parseOpenScad3mf } = await import('../src/exchange/openScadImport')
    const { OpenScadProject } = await import('../src/language/openScadProject')

    const extract3mfXml = (zipBytes: Uint8Array) => {
      const view = new DataView(zipBytes.buffer, zipBytes.byteOffset, zipBytes.byteLength)
      const dirOff = view.getUint32(zipBytes.length - 6, true)
      const count = view.getUint16(zipBytes.length - 12, true)
      let cursor = dirOff
      for (let i = 0; i < count; i++) {
        const method = view.getUint16(cursor + 10, true)
        const compSize = view.getUint32(cursor + 20, true)
        const nameLen = view.getUint16(cursor + 28, true)
        const extraLen = view.getUint16(cursor + 30, true)
        const commentLen = view.getUint16(cursor + 32, true)
        const localOff = view.getUint32(cursor + 42, true)
        const name = new TextDecoder().decode(zipBytes.subarray(cursor + 46, cursor + 46 + nameLen))
        if (name === '3D/3dmodel.model') {
          const lNameLen = view.getUint16(localOff + 26, true)
          const lExtraLen = view.getUint16(localOff + 28, true)
          const start = localOff + 30 + lNameLen + lExtraLen
          const payload = zipBytes.subarray(start, start + compSize)
          return new TextDecoder().decode(method === 8 ? inflateRawSync(payload) : payload)
        }
        cursor += 46 + nameLen + extraLen + commentLen
      }
      throw new Error('Missing 3D/3dmodel.model')
    }

    for (const [part, hookPrefix] of [
      [0, 'skadis_hook_'],
      [3, 'skadis_print_hook_'],
    ] as const) {
      const source = generateSkadisVoronoiBox({ ...SKADIS_VORONOI_BOX_DEFAULTS, width: 240, skadis_part: part })
      const { result } = await engine.buildSource(source, request)
      const artifact = await exportMesh(result.meshes, '3mf', {
        quality: 'viewport',
        source,
        title: 'skadis-voronoi-box',
      })
      const xml = extract3mfXml(artifact.data)

      expect(xml).toContain('displaycolor="#2E8CA3"')
      expect(xml).toContain('displaycolor="#F2A638"')
      expect(xml).toContain('name="floor_slab"')
      expect(xml).toContain('name="front_panel"')
      expect(xml).toContain('name="front_panel_gusset"')
      expect(xml).toContain('name="left_panel"')
      expect(xml).toContain('name="right_panel"')
      expect(xml).toContain('name="back_bay_1"')
      expect(xml).toContain('name="mount_column_1"')
      expect(xml).toContain('name="bent_corner_1"')
      expect(xml).toContain('name="corner_gusset_1"')
      expect(xml).toContain('name="skadis_body" type="model"><components>')
      expect(xml).toContain(`name="${hookPrefix}1"`)
      expect(xml).toContain(`name="${hookPrefix}2"`)
      expect(xml).toContain(`name="${hookPrefix}3"`)
      expect(xml.match(/<item objectid=/g)).toHaveLength(4)

      const project = new OpenScadProject({
        entrypoint: 'main.scad',
        files: [
          { kind: 'source', path: 'main.scad', source: '' },
          { kind: 'blob', path: 'export.3mf', data: artifact.data },
        ],
      })
      const parsed = await parseOpenScad3mf(project.read('export.3mf')!)
      const expectedTris = result.meshes.reduce((sum, m) => sum + m.indices.length / 3, 0)
      expect(parsed.triangleCount).toBe(expectedTris)
    }
  }, 120000)
})
