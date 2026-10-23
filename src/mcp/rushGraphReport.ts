import { evaluateRushGraphGeometry } from '../services/rushGraphChecks'
import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import type { McpGeometryService } from './geometryService'
import { compileRushGraph, rushGraphSchema } from '../services/rushGraph'
import { renderRushGraphPreviews } from './rushGraphPreview'

function buildLineStarts(source: string): number[] {
  const starts = [0]
  for (let i = 0; i < source.length; i++) {
    if (source.charCodeAt(i) === 10) starts.push(i + 1)
  }
  return starts
}

function lineNumberAtOffset(lineStarts: number[], offset: number): number {
  let lo = 0
  let hi = lineStarts.length - 1
  while (lo <= hi) {
    const mid = (lo + hi) >>> 1
    if (lineStarts[mid] <= offset) lo = mid + 1
    else hi = mid - 1
  }
  return Math.max(1, lo)
}

function findSourceMapEntry<T extends { line: number }>(sourceMap: readonly T[], line: number): T | undefined {
  let lo = 0
  let hi = sourceMap.length - 1
  let best: T | undefined
  while (lo <= hi) {
    const mid = (lo + hi) >>> 1
    if (sourceMap[mid].line <= line) {
      best = sourceMap[mid]
      lo = mid + 1
    } else {
      hi = mid - 1
    }
  }
  return best
}

export function registerRushGraphReport(server: McpServer, geometry: McpGeometryService) {
  server.registerTool('rush_report', {
    description: 'Build actual geometry and return measurements, source-to-node provenance and front/top/isometric PNG images. Preview limits may produce images_status=unavailable while measurements remain available. Does not certify printability.',
    inputSchema: z.object({ document: rushGraphSchema }), annotations: { readOnlyHint: true, openWorldHint: false },
  }, async (input, context) => {
    try {
      const compiled = compileRushGraph(input.document)
      const built = await geometry.compile(compiled.source, 'full', context.mcpReq.signal)
      const checks = await evaluateRushGraphGeometry(compiled.geometry_assertions, built.meshes, async source => (await geometry.compile(source, 'full', context.mcpReq.signal)).meshes)
      const lineStarts = buildLineStarts(compiled.source)
      const references: Array<{
        mesh: number
        triangle_start: number
        triangle_end: number
        node_id: string | null
        instance_path: string | null
      }> = []
      let totalReferences = 0
      for (let meshIndex = 0; meshIndex < built.meshes.length; meshIndex++) {
        const mesh = built.meshes[meshIndex]!
        for (let r = 0; r < mesh.provenance.length; r++) {
          totalReferences++
          if (references.length >= 2048) continue
          const run = mesh.provenance[r]!
          let line: number | null = null
          if (run.source) {
            const end = Math.min(run.source.start, compiled.source.length)
            line = lineNumberAtOffset(lineStarts, end)
          }
          const match = line !== null ? findSourceMapEntry(compiled.source_map, line) : undefined
          references.push({
            mesh: meshIndex,
            triangle_start: run.triangleStart,
            triangle_end: run.triangleEnd,
            node_id: match?.node_id ?? null,
            instance_path: match?.instance_path ?? null,
          })
        }
      }
      let previews: ReturnType<typeof renderRushGraphPreviews> = [], imageError: string | null = null
      try { previews = renderRushGraphPreviews(built.meshes) } catch (error) { imageError = error instanceof Error ? error.message : 'Preview unavailable' }
      const report = { checks, document_sha256: compiled.document_sha256, analysis: built.analysis, constraints: compiled.constraint_report, sketch_solutions: compiled.sketch_solutions, assembly_components: compiled.assembly_components, mechanical_reports: compiled.mechanical_reports, mechanical_parts: compiled.mechanical_parts, references, references_truncated: totalReferences > 2048, reference_scope: 'generated-operation provenance; not persistent face identifiers', images_status: imageError ? 'unavailable' : 'rendered', image_error: imageError, views: previews.map(item => item.view), printability: 'unknown' }
      return { isError: checks.some(c => c.status !== 'passed'), structuredContent: report, content: [{ type: 'text' as const, text: JSON.stringify(report) }, ...previews.map(item => ({ type: 'image' as const, mimeType: 'image/png', data: item.png.toString('base64') }))] }
    } catch { return { isError: true, content: [{ type: 'text' as const, text: 'Report failed. Call rush_compile and rush_check for diagnostics.' }] } }
  })
}

export interface EngineeringSummaryReportInput {
  nodeCount: number
  bodyCount: number
  totalVolumeMm3?: number
  totalSurfaceAreaMm2?: number
  warnings?: string[]
}

/**
 * Generates a structured Markdown engineering summary report for LLM agents
 * inspecting a compiled CAD / Rush Graph model.
 */
export function formatEngineeringSummaryMarkdown(report: EngineeringSummaryReportInput): string {
  const lines: string[] = [
    '# Engineering Summary Report',
    '',
    '| Metric | Value |',
    '| :--- | :--- |',
    `| **Graph Nodes** | ${report.nodeCount} |`,
    `| **Solid Bodies** | ${report.bodyCount} |`,
  ]

  if (report.totalVolumeMm3 !== undefined && Number.isFinite(report.totalVolumeMm3)) {
    const cm3 = report.totalVolumeMm3 / 1000
    lines.push(`| **Total Volume** | ${report.totalVolumeMm3.toFixed(2)} mm³ (${cm3.toFixed(3)} cm³) |`)
  } else {
    lines.push('| **Total Volume** | N/A |')
  }

  if (report.totalSurfaceAreaMm2 !== undefined && Number.isFinite(report.totalSurfaceAreaMm2)) {
    const cm2 = report.totalSurfaceAreaMm2 / 100
    lines.push(`| **Total Surface Area** | ${report.totalSurfaceAreaMm2.toFixed(2)} mm² (${cm2.toFixed(2)} cm²) |`)
  } else {
    lines.push('| **Total Surface Area** | N/A |')
  }

  const warnings = report.warnings ?? []
  lines.push(`| **Warnings** | ${warnings.length} |`)

  if (warnings.length > 0) {
    lines.push('', '## Warnings', '')
    for (const warning of warnings) {
      lines.push(`- ⚠️ ${warning}`)
    }
  } else {
    lines.push('', '_Status: Clean — 0 warnings._')
  }

  return lines.join('\n')
}
