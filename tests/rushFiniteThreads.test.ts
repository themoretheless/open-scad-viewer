import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

for (const kind of ['external', 'internal']) {
  it(`preserves the original walls and finite caps on the ${kind} thread`, async () => {
    const source = readFileSync(`examples/rush/finite-${kind}-thread.r`, 'utf8')
    const scene = await buildTextNurbsScene(compileRushFrontend(source).document)
    const report = JSON.parse(scene.measurements![0]!.report)
    expect(report.patchErrorUpperMm).toBeGreaterThan(0)
    expect(report.patchErrorUpperMm).toBeLessThanOrEqual(0.01)
    expect(report.axialErrorBandUpperMm).toBeGreaterThanOrEqual(0)
    expect(report.boundaryAgreement).toBe(true)
    expect(report.originalWallCount).toBeGreaterThan(0)
    const model = JSON.parse(scene.meshes[0]!.nativeGeometry!.geometryJson).geometry
    expect(model.bodies).toHaveLength(1)
    expect(model.shells[0].closed).toBe(true)
    expect(scene.volume).toBeGreaterThan(0)
  }, 180000)
}
