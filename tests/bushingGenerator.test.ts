import { beforeAll, describe, expect, it } from 'vitest'
import {
  BUSHING_DEFAULTS,
  computeBushingParameterSuggestions,
  generateBushing,
} from '../src/services/bushingGenerator'
import { extractCustomizerParameters } from '../src/language/scadCustomizer'
import { extractSemanticEdges } from '../src/kernel/meshTopology'
import { GeometryBuildEngine } from '../src/kernel/geometryBuildEngine'

const engine = new GeometryBuildEngine()

describe('threaded bushing generator', () => {
  beforeAll(async () => {
    await engine.initializeSource('cube(1);', { quality: 'full', purpose: 'analysis' })
  }, 120000)

  it('extracts all customizer parameters including select options and numeric ranges', () => {
    const source = generateBushing(BUSHING_DEFAULTS)
    const params = extractCustomizerParameters(source)
    expect(params.length).toBe(Object.keys(BUSHING_DEFAULTS).length)
    const modeParam = params.find(p => p.name === 'thread_mode')
    expect(modeParam?.options).toEqual([0, 1, 2, 3])
    const shapeParam = params.find(p => p.name === 'outer_shape')
    expect(shapeParam?.options).toEqual([0, 1, 2, 3, 4])
    const flangeShapeParam = params.find(p => p.name === 'flange_shape')
    expect(flangeShapeParam?.options).toEqual([0, 1, 2])
  })

  it('builds a closed watertight manifold for all 4 thread modes (internal, external, dual reducer, female-bottom/male-top tube)', async () => {
    const defaultSource = generateBushing(BUSHING_DEFAULTS)
    const defaultBuild = await engine.buildSource(defaultSource, { quality: 'full', purpose: 'analysis' })
    expect(defaultBuild.result.volume).toBeGreaterThan(100)
    expect(defaultBuild.result.meshes.length).toBe(1)

    for (const thread_mode of [0, 1, 2, 3] as const) {
      const source = generateBushing({
        ...BUSHING_DEFAULTS,
        thread_mode,
        segments: 24,
      })
      const { result } = await engine.buildSource(source, { quality: 'full', purpose: 'analysis' })
      expect(result.volume).toBeGreaterThan(100)
      expect(result.meshes.length).toBe(1)
      for (const mesh of result.meshes) {
        const { diagnostics } = extractSemanticEdges(mesh.vertices, mesh.indices)
        expect(diagnostics.boundary).toBe(0)
        expect(diagnostics.nonManifold).toBe(0)
      }
    }
  }, 120000)

  it('supports all outer profiles (round, hex, wrench flats, square, knurled splines) and flange holes', async () => {
    for (const outer_shape of [0, 1, 2, 3, 4] as const) {
      const source = generateBushing({
        ...BUSHING_DEFAULTS,
        outer_shape,
        flange: true,
        flange_diameter: 30,
        flange_holes: outer_shape === 1 ? 4 : 0,
        flange_hole_diameter: 2.8,
        segments: 24,
      })
      const { result } = await engine.buildSource(source, { quality: 'full', purpose: 'analysis' })
      expect(result.volume).toBeGreaterThan(100)
      expect(result.meshes.length).toBe(1)
      for (const mesh of result.meshes) {
        const { diagnostics } = extractSemanticEdges(mesh.vertices, mesh.indices)
        expect(diagnostics.boundary).toBe(0)
        expect(diagnostics.nonManifold).toBe(0)
      }
    }
  }, 120000)

  it('supports blind bottom, partial thread depth, cross-hole, left-handed multi-start and split collet slot', async () => {
    const blindAndCross = generateBushing({
      ...BUSHING_DEFAULTS,
      thread_mode: 2,
      auto_thread_std: false,
      fdm_thread_angle: false,
      inner_starts: 2,
      inner_left_handed: true,
      inner_thread_depth: 10,
      outer_starts: 2,
      outer_left_handed: false,
      blind_bottom: 3,
      cross_hole_diameter: 3,
      cross_hole_z: 10,
      segments: 24,
    })
    const res1 = await engine.buildSource(blindAndCross, { quality: 'full', purpose: 'analysis' })
    expect(res1.result.volume).toBeGreaterThan(100)
    for (const mesh of res1.result.meshes) {
      const { diagnostics } = extractSemanticEdges(mesh.vertices, mesh.indices)
      expect(diagnostics.boundary).toBe(0)
      expect(diagnostics.nonManifold).toBe(0)
    }

    const splitCollet = generateBushing({
      ...BUSHING_DEFAULTS,
      flange: false,
      split_slot_width: 1.5,
      segments: 24,
    })
    const res2 = await engine.buildSource(splitCollet, { quality: 'full', purpose: 'analysis' })
    expect(res2.result.volume).toBeGreaterThan(50)
    for (const mesh of res2.result.meshes) {
      const { diagnostics } = extractSemanticEdges(mesh.vertices, mesh.indices)
      expect(diagnostics.boundary).toBe(0)
      expect(diagnostics.nonManifold).toBe(0)
    }
  }, 120000)

  it('rejects impossible walls, invalid flange dimensions, and out-of-bounds cuts', () => {
    expect(() => generateBushing({ ...BUSHING_DEFAULTS, inner_diameter: 20, outer_diameter: 22 })).toThrow(/стенки|wall/)
    expect(() => generateBushing({ ...BUSHING_DEFAULTS, thread_mode: 3, length: 18, thread_section_length: 10 })).toThrow(/Длина|length/)
    expect(() => generateBushing({ ...BUSHING_DEFAULTS, thread_mode: 0, auto_thread_std: false, inner_diameter: 3, inner_pitch: 2 })).toThrow(/шага|pitch/)
    expect(() => generateBushing({ ...BUSHING_DEFAULTS, flange: true, flange_diameter: 18, outer_diameter: 22 })).toThrow(/буртика|flange_diameter/)
    expect(() => generateBushing({ ...BUSHING_DEFAULTS, flange: true, flange_holes: 4, flange_diameter: 24, outer_diameter: 22, flange_hole_diameter: 3 })).toThrow(/отверстия|holes/)
    expect(() => generateBushing({ ...BUSHING_DEFAULTS, thread_mode: 0, blind_bottom: 39.5, length: 40 })).toThrow(/глухого|blind_bottom/)
    expect(() => generateBushing({ ...BUSHING_DEFAULTS, cross_hole_diameter: 4, cross_hole_z: 1.5 })).toThrow(/Поперечное|Cross-hole/)
    expect(() => generateBushing({ ...BUSHING_DEFAULTS, split_slot_width: 11.5, inner_diameter: 12 })).toThrow(/разреза|split_slot_width/)
  })

  it('suggests matching outer diameter, standard metric pitch and FDM clearance when inner diameter increases', () => {
    const suggestions = computeBushingParameterSuggestions('inner_diameter', {
      ...BUSHING_DEFAULTS,
      thread_mode: 3,
      inner_diameter: 16,
      outer_diameter: 20,
      inner_pitch: 1.0,
    })
    expect(suggestions.length).toBeGreaterThan(0)
    expect(suggestions[0].changes.outer_diameter).toBeGreaterThan(20)
    expect(suggestions[0].changes.inner_pitch).toBe(2.0)
  })
})
