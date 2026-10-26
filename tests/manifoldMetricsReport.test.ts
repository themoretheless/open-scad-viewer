import { describe, expect, it } from 'vitest'
import type { KernelManifoldMetrics } from '../src/services/geometry/manifoldAnalysis'
import { serializeManifoldMetricsReport } from '../src/services/manifoldMetricsReport'

const metrics = (): KernelManifoldMetrics => ({
  vertexCount: 8,
  triangleCount: 12,
  edgeCount: 18,
  eulerCharacteristic: 2,
  componentCount: 2,
  signedVolume: 48.12345678901234,
  surfaceArea: 88.98765432109876,
  watertight: false,
  components: [
    { triangleCount: 8, vertexCount: 6, edgeCount: 12, boundaryEdges: 0, eulerCharacteristic: 2, genus: 0, signedVolume: 40, surfaceArea: 70 },
    { triangleCount: 4, vertexCount: 2, edgeCount: 6, boundaryEdges: 2, eulerCharacteristic: 0, genus: null, signedVolume: 8.12345678901234, surfaceArea: 18.98765432109876 },
  ],
})

describe('manifold metrics report', () => {
  it('exports exact values and per-component topology to versioned JSON', () => {
    const text = serializeManifoldMetricsReport([{ id: 'body-1', name: 'Body', metrics: metrics() }], 'json')
    const report = JSON.parse(text)
    expect(report).toMatchObject({
      format: 'open-scad-viewer.manifold-metrics', version: 1, source: 'current-display-mesh',
      units: { signedVolume: 'mm^3', surfaceArea: 'mm^2' },
    })
    expect(report.bodies[0]).toMatchObject({
      id: 'body-1', signedVolumeMm3: 48.12345678901234, surfaceAreaMm2: 88.98765432109876,
      componentCount: 2, watertight: false,
    })
    expect(report.bodies[0].components.map((component: { genus: number | null }) => component.genus)).toEqual([0, null])
    expect(text.endsWith('\n')).toBe(true)
  })

  it('exports selected bodies as CSV rows with escaped names and explicit unknown genus', () => {
    const second = metrics()
    second.componentCount = 1
    second.components = [second.components[0]]
    second.watertight = true
    const text = serializeManifoldMetricsReport([
      { id: 'a', name: 'Body, "A"', metrics: metrics() },
      { id: 'b', name: 'Line\nbreak', metrics: second },
    ], 'csv')
    const lines = text.split('\r\n')
    expect(lines[0]).toBe('bodyId,bodyName,vertexCount,triangleCount,edgeCount,signedVolumeMm3,surfaceAreaMm2,eulerCharacteristic,componentCount,watertight,componentGenera')
    expect(lines[1]).toBe('a,"Body, ""A""",8,12,18,48.12345678901234,88.98765432109876,2,2,false,0;null')
    expect(lines[2]).toBe('b,"Line\nbreak",8,12,18,48.12345678901234,88.98765432109876,2,1,true,0')
    expect(text.endsWith('\r\n')).toBe(true)
  })

  it('refuses an empty selection instead of creating a misleading report', () => {
    expect(() => serializeManifoldMetricsReport([], 'json')).toThrow('at least one body')
    expect(() => serializeManifoldMetricsReport([], 'csv')).toThrow('at least one body')
  })
})
