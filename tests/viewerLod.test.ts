import { describe, expect, it } from 'vitest'
import { pixelsPerUnit, selectViewerLod, type ViewerLodCamera } from '../src/render/viewerLod'

const camera = (overrides: Partial<ViewerLodCamera> = {}): ViewerLodCamera => ({
  eye: [0, 0, 10],
  distance: 10,
  eyeDistance: 10,
  fovY: Math.PI / 2,
  projection: 'perspective',
  viewportHeightPx: 1000,
  ...overrides,
})

describe('viewer LOD screen scale', () => {
  it('matches the projection at the orbit target', () => {
    // halfHeight = 10 * tan(45deg) = 10, so 1000 px span 20 units at the target.
    expect(pixelsPerUnit(camera(), [0, 0, 0], 0)).toBeCloseTo(50, 12)
  })

  it('grows toward the eye and uses the nearest sphere point', () => {
    expect(pixelsPerUnit(camera(), [0, 0, 5], 0)).toBeCloseTo(100, 12)
    expect(pixelsPerUnit(camera(), [0, 0, 0], 5)).toBeCloseTo(100, 12)
  })

  it('is depth independent in orthographic projection', () => {
    const ortho = camera({ projection: 'orthographic' })
    expect(pixelsPerUnit(ortho, [0, 0, 0], 0)).toBeCloseTo(50, 12)
    expect(pixelsPerUnit(ortho, [0, 0, -100], 3)).toBeCloseTo(50, 12)
  })

  it('stays finite when the eye is inside the bounding sphere', () => {
    const scale = pixelsPerUnit(camera(), [0, 0, 9], 5)
    expect(Number.isFinite(scale)).toBe(true)
    expect(scale).toBeGreaterThan(1000)
  })

  it('refuses invalid inputs', () => {
    expect(() => pixelsPerUnit(camera({ viewportHeightPx: 0 }), [0, 0, 0], 1)).toThrow(RangeError)
    expect(() => pixelsPerUnit(camera(), [0, 0, 0], Number.NaN)).toThrow(RangeError)
  })
})

describe('native viewer LOD selection', () => {
  const levels = [
    { triangleCount: 100, errorUpper: 0.4 },
    { triangleCount: 400, errorUpper: 0.1 },
    { triangleCount: 1600, errorUpper: 0.025 },
  ]
  const request = (pixelsPerUnit: number) => ({
    pixelsPerUnit,
    pixelBudget: 1,
    triangleBudget: 100_000,
    hysteresis: 1.5,
  })

  it('selects through the installed kernel and holds inside the hysteresis band', () => {
    expect(selectViewerLod(levels, null, request(20))).toMatchObject({
      level: 2,
      reason: 'initial',
      withinPixelBudget: true,
    })
    expect(selectViewerLod(levels, 1, request(12))).toMatchObject({ level: 1, reason: 'hysteresis-hold' })
    expect(selectViewerLod(levels, 2, request(0.5))).toMatchObject({ level: 1, reason: 'pixel-budget' })
  })

  it('refuses a single level', () => {
    expect(() => selectViewerLod(levels.slice(0, 1), null, request(1))).toThrow()
  })
})
