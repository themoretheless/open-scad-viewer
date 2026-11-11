import type { KernelManifoldMetrics } from './geometry/manifoldAnalysis'

export type ManifoldMetricsExportFormat = 'json' | 'csv'
export interface ManifoldMetricsReportRow {
  id: string
  name: string
  metrics: KernelManifoldMetrics
}

/** Stable, unit-explicit export of the current display meshes, not a B-rep certificate. */
export function createManifoldMetricsReport(rows: readonly ManifoldMetricsReportRow[]) {
  return {
    format: 'open-scad-viewer.manifold-metrics',
    version: 1,
    source: 'current-display-mesh',
    units: { signedVolume: 'mm^3', surfaceArea: 'mm^2' },
    bodies: rows.map(({ id, name, metrics }) => ({
      id,
      name,
      vertexCount: metrics.vertexCount,
      triangleCount: metrics.triangleCount,
      edgeCount: metrics.edgeCount,
      signedVolumeMm3: metrics.signedVolume,
      surfaceAreaMm2: metrics.surfaceArea,
      eulerCharacteristic: metrics.eulerCharacteristic,
      componentCount: metrics.componentCount,
      watertight: metrics.watertight,
      components: metrics.components.map((component, index) => ({
        component: index + 1,
        vertexCount: component.vertexCount,
        triangleCount: component.triangleCount,
        edgeCount: component.edgeCount,
        boundaryEdges: component.boundaryEdges,
        eulerCharacteristic: component.eulerCharacteristic,
        genus: component.genus,
        signedVolumeMm3: component.signedVolume,
        surfaceAreaMm2: component.surfaceArea,
      })),
    })),
  }
}

const CSV_COLUMNS = [
  'bodyId', 'bodyName', 'vertexCount', 'triangleCount', 'edgeCount',
  'signedVolumeMm3', 'surfaceAreaMm2', 'eulerCharacteristic',
  'componentCount', 'watertight', 'componentGenera',
] as const

function csvCell(value: string | number | boolean): string {
  const text = String(value)
  return /[",\r\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text
}

export function serializeManifoldMetricsReport(
  rows: readonly ManifoldMetricsReportRow[],
  format: ManifoldMetricsExportFormat,
): string {
  if (!rows.length) throw new Error('Select at least one body for the metrics report.')
  const report = createManifoldMetricsReport(rows)
  if (format === 'json') return `${JSON.stringify(report, null, 2)}\n`
  const lines = [CSV_COLUMNS.join(',')]
  for (const body of report.bodies) {
    const componentGenera = body.components.map(component => component.genus === null ? 'null' : component.genus).join(';')
    lines.push([
      body.id, body.name, body.vertexCount, body.triangleCount, body.edgeCount,
      body.signedVolumeMm3, body.surfaceAreaMm2, body.eulerCharacteristic,
      body.componentCount, body.watertight, componentGenera,
    ].map(csvCell).join(','))
  }
  return `${lines.join('\r\n')}\r\n`
}
