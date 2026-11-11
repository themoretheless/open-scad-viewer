import { ArtifactSizeError } from '../../geometryService'
import { independentArtifactSchema, independentCheckInputSchema, independentCheckSchema, independentExportInputSchema, independentExportSchema } from '../../createServer'
import { INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION, executeIndependentOpenScad, independentOpenScadMetrics } from '../../independentOpenScadExecution'
import { independentArtifactMimeType } from '../../independentArtifactCache'
import { independentArtifactToWire, sha256, toolOutputSchema } from '../officialToolInputSchema'
import { buildBinaryStl, buildObj } from '../../../exchange/meshExport'
import { errorResult, estimatedIndependentExportBytes, safeFileName, textResult, throwIfIndependentExportAborted } from '../persistedFailureStatus'
import { z } from 'zod/v4'
import type { McpServerContext } from './context'

/** Independent stable-2021.01 evaluator: check and full-quality export. */
export function registerIndependentTools(ctx: McpServerContext): void {
  const { geometry, independentArtifacts, server, notifyResourceListChanged } = ctx
  server.registerTool('openscad_independent_check', {
    title: 'Check with the independent OpenSCAD engine',
    description: 'Execute an inline source or bounded include/use project with the project-owned OpenSCAD 2021.01 frontend and geometry pipeline, without invoking the upstream OpenSCAD runtime. This remains a development surface, not a complete-language production claim.',
    inputSchema: independentCheckInputSchema,
    outputSchema: toolOutputSchema(z.object({ check: independentCheckSchema }).strict()),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: false, openWorldHint: false },
  }, async (input, context) => {
    try {
      const run = await executeIndependentOpenScad({
        source: input.source,
        files: input.files,
        quality: input.quality,
        time: input.time,
        signal: context.mcpReq.signal,
      })
      return textResult({
        check: {
          engine: INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION,
          source_sha256: sha256(input.source),
          quality: run.result.quality,
          duration_ms: run.durationMs,
          ...independentOpenScadMetrics(run.result),
          warnings: run.result.warnings,
        },
      })
    } catch (error) {
      return errorResult(error)
    }
  })

  server.registerTool('openscad_independent_export', {
    title: 'Export with the independent OpenSCAD engine',
    description: 'Render an inline source or bounded VFS project at full quality with only the repository-owned OpenSCAD 2021.01 engine, then expose a bounded session-local STL/OBJ resource. No upstream runtime or legacy DuckDB build record is used.',
    inputSchema: independentExportInputSchema,
    outputSchema: toolOutputSchema(z.object({
      export: independentExportSchema,
      artifact: independentArtifactSchema,
    }).strict()),
    annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async (input, context) => {
    const startedAt = performance.now()
    try {
      const run = await executeIndependentOpenScad({
        source: input.source,
        files: input.files,
        quality: 'full',
        time: input.time,
        signal: context.mcpReq.signal,
      })
      throwIfIndependentExportAborted(context.mcpReq.signal)
      const estimatedBytes = estimatedIndependentExportBytes(run.result.meshes, input.format)
      if (estimatedBytes > input.max_bytes) {
        throw new ArtifactSizeError(estimatedBytes, input.max_bytes)
      }

      const fileName = safeFileName(input.file_name ?? 'model', input.format)
      const mimeType = independentArtifactMimeType(input.format)
      const data = input.format === 'stl'
        ? buildBinaryStl(run.result.meshes, fileName)
        : new TextEncoder().encode(buildObj(run.result.meshes))
      throwIfIndependentExportAborted(context.mcpReq.signal)
      if (data.byteLength > input.max_bytes) {
        throw new ArtifactSizeError(data.byteLength, input.max_bytes)
      }

      const dataDigest = sha256(data)
      const artifact = independentArtifacts.put({
        format: input.format,
        fileName,
        mimeType,
        data,
      })
      if (artifact.id !== dataDigest
        || artifact.sha256 !== dataDigest
        || artifact.byteLength !== data.byteLength
        || artifact.format !== input.format
        || artifact.mimeType !== mimeType
        || artifact.fileName !== fileName) {
        throw new Error('Independent OpenSCAD export failed its MCP integrity boundary')
      }
      notifyResourceListChanged()
      const artifactWire = independentArtifactToWire(artifact)
      const structured = {
        export: {
          engine: INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION,
          source_sha256: sha256(input.source),
          quality: 'full' as const,
          format: input.format,
          duration_ms: Math.max(0, performance.now() - startedAt),
          metrics: independentOpenScadMetrics(run.result),
          warnings: run.result.warnings,
        },
        artifact: artifactWire,
      }
      const result = textResult(structured)
      return {
        ...result,
        content: [
          result.content[0],
          {
            type: 'resource_link' as const,
            uri: artifactWire.resource_uri,
            name: artifact.fileName,
            mimeType: artifact.mimeType,
            description: `Independent OpenSCAD ${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString('en-US')} bytes); session-local`,
          },
        ],
      }
    } catch (error) {
      return errorResult(error)
    }
  })
}
