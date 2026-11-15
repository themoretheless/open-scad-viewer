import { DEFAULT_MAX_ARTIFACT_BYTES, GeometryBusyError, MAX_ARTIFACT_BYTES } from '../../geometryService'
import { analysisSchema, buildSchema, idSchema } from '../../createServer'
import type { GeometryExecutionDescriptor } from '../../../core/geometryExecution'
import { isWellFormedUnicode } from '../../modelStore'
import type { StoredBuild } from '../../modelStore'
import { analysisToWire, artifactUri, selectorSchema, sha256, toolOutputSchema } from '../officialToolInputSchema'
import { attachGeometryExecutionToError } from '../../../kernel/geometryBuildEngine'
import {
  buildToWire,
  errorResult,
  recordFailure,
  resolveSource,
  safeFileName,
  textResult,
} from '../persistedFailureStatus'
import { z } from 'zod/v4'
import type { McpServerContext } from './context'

/** Artifact export and persisted build history tools. */
export function registerExportAndHistoryTools(ctx: McpServerContext): void {
  registerOpenscadExport(ctx)
  registerOpenscadBuildHistory(ctx)
}

function registerOpenscadExport(ctx: McpServerContext): void {
  const { store, geometry, server, notifyResourceListChanged } = ctx
  server.registerTool(
    'openscad_export',
    {
      title: 'Export OpenSCAD geometry',
      description:
        'Compile a full-quality model, store a bounded STL, OBJ, or AP242 STEP (B-rep only) artifact in DuckDB, and return an MCP resource link.',
      inputSchema: selectorSchema({
        format: z.enum(['stl', 'obj', 'step']),
        file_name: z
          .string()
          .min(1)
          .max(255)
          .refine(isWellFormedUnicode, 'File name must contain well-formed Unicode')
          .optional(),
        max_bytes: z.number().int().positive().max(MAX_ARTIFACT_BYTES).default(DEFAULT_MAX_ARTIFACT_BYTES),
      }),
      outputSchema: toolOutputSchema(
        z.object({
          build: buildSchema,
          analysis: analysisSchema,
          artifact: z.object({
            id: z.string(),
            resource_uri: z.string(),
            file_name: z.string(),
            format: z.enum(['stl', 'obj', 'step']),
            mime_type: z.string(),
            sha256: z.string(),
            byte_length: z.number().int().nonnegative(),
          }),
        }),
      ),
      annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false },
    },
    async (input, context) => {
      const startedAt = performance.now()
      let completedBuild: StoredBuild | null = null
      let completedExecution: GeometryExecutionDescriptor | undefined
      let selected: Awaited<ReturnType<typeof resolveSource>>
      try {
        selected = await resolveSource(store, input)
      } catch (error) {
        return errorResult(error)
      }
      try {
        const exported = await geometry.export(selected.source, input.format, {
          maxBytes: input.max_bytes,
          signal: context.mcpReq.signal,
          name: input.file_name ?? selected.model?.name,
        })
        const analysis = exported.analysis
        completedExecution = analysis.execution
        let build: StoredBuild
        try {
          build = await store.recordBuild({
            modelId: selected.model?.id,
            modelRevision: selected.model?.revision,
            source: selected.source,
            sourceSha256: sha256(selected.source),
            execution: analysis.execution,
            quality: 'full',
            durationMs: analysis.durationMs,
            warnings: analysis.warnings,
            status: 'succeeded',
            metrics: {
              meshCount: analysis.meshCount,
              triangleCount: analysis.triangleCount,
              volume: analysis.volume,
              surfaceArea: analysis.surfaceArea,
              reduced: analysis.reduced,
            },
          })
        } catch (error) {
          attachGeometryExecutionToError(error, analysis.execution)
          throw error
        }
        completedBuild = build
        const fileName = safeFileName(input.file_name ?? selected.model?.name ?? 'model', input.format)
        const artifact = await store.storeArtifact({
          buildId: build.id,
          modelId: selected.model?.id,
          format: input.format,
          fileName,
          mimeType: exported.mimeType,
          sha256: sha256(exported.data),
          data: exported.data,
        })
        notifyResourceListChanged()
        const structured = {
          build: buildToWire(build),
          analysis: analysisToWire(analysis),
          artifact: {
            id: artifact.id,
            resource_uri: artifactUri(artifact.id),
            file_name: artifact.fileName,
            format: artifact.format,
            mime_type: artifact.mimeType,
            sha256: artifact.sha256,
            byte_length: artifact.byteLength,
          },
        }
        const result = textResult(structured)
        return {
          ...result,
          content: [
            result.content[0],
            {
              type: 'resource_link' as const,
              uri: artifactUri(artifact.id),
              name: artifact.fileName,
              mimeType: artifact.mimeType,
              description: `${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString('en-US')} bytes)`,
            },
          ],
        }
      } catch (error) {
        if (completedBuild) {
          notifyResourceListChanged()
          return errorResult(error, { build: buildToWire(completedBuild) })
        }
        if (error instanceof GeometryBusyError) return errorResult(error)
        const build = await recordFailure(store, geometry, {
          ...selected,
          quality: 'full',
          purpose: 'export',
          durationMs: performance.now() - startedAt,
          error,
          execution: completedExecution,
        })
        if (build) notifyResourceListChanged()
        return errorResult(error, build ? { build: buildToWire(build) } : {})
      }
    },
  )
}

function registerOpenscadBuildHistory(ctx: McpServerContext): void {
  const { store, server } = ctx
  server.registerTool(
    'openscad_build_history',
    {
      title: 'OpenSCAD build history',
      description: 'List recent MCP build results recorded in DuckDB, optionally for one saved model.',
      inputSchema: z.object({
        model_id: idSchema.optional(),
        limit: z.number().int().min(1).max(100).default(100),
      }),
      outputSchema: toolOutputSchema(z.object({ builds: z.array(buildSchema) })),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
    },
    async input => {
      try {
        const builds = await store.listBuilds({ modelId: input.model_id, limit: input.limit })
        return textResult({ builds: builds.map(buildToWire) })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}
