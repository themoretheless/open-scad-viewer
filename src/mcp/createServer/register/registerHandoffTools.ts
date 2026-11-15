import { z } from 'zod/v4'
import {
  createOpenScadHandoffBundle,
  HandoffConflictError,
  OPENSCAD_HANDOFF_BUNDLE_SCHEMA,
  verifyOpenScadHandoffBundle,
} from '../../../core/handoffBundle'
import { ModelNotFoundError } from '../../publicError'
import { modelToWire, selectorSchema, toolOutputSchema } from '../officialToolInputSchema'
import { errorResult, textResult } from '../persistedFailureStatus'
import { idSchema, modelMetadataSchema, nameSchema, scalarSchema, sha256Schema } from '../schemas/commonSchemas'
import type { McpServerContext } from './context'

const parameterPresetsSchema = z
  .record(z.string(), z.record(z.string(), scalarSchema))
  .refine(presets => Object.keys(presets).length <= 32, 'parameter_presets is limited to 32 presets')

const handoffRoutingSchema = z.object({
  language_contract: z.enum(['legacy/current', 'openscad-viewer/brep-1']),
  engine_class: z.enum(['mesh', 'brep']),
  required_capabilities: z.array(z.string()),
  automatic_fallback: z.literal(false),
})

const handoffBundleSchema = z.object({
  schema: z.literal(OPENSCAD_HANDOFF_BUNDLE_SCHEMA),
  origin: z.enum(['browser-workspace', 'mcp-catalog']),
  created_at: z.string(),
  document: z.object({
    id: z.string().nullable(),
    name: z.string(),
    source: z.string(),
    source_sha256: sha256Schema,
    revision: z.number().int().nonnegative().nullable(),
    parameter_presets: parameterPresetsSchema.optional(),
  }),
  routing: handoffRoutingSchema,
  bundle_sha256: sha256Schema,
})

// Accept structurally typed JSON objects at MCP validation time so verifyOpenScadHandoffBundle
// surfaces tampered digests or routing headers as structured PublicToolError codes.
const looseHandoffBundleInputSchema = z
  .object({
    schema: z.string(),
    origin: z.string(),
    created_at: z.string(),
    document: z.record(z.string(), z.unknown()),
    routing: z.record(z.string(), z.unknown()),
    bundle_sha256: z.string(),
  })
  .passthrough()

/** Explicit verified browser <-> MCP handoff bundle tools with strict CAS on import. */
export function registerHandoffTools(ctx: McpServerContext): void {
  const { store, geometry, server, notifyResourceListChanged } = ctx

  server.registerTool(
    'openscad_export_handoff_bundle',
    {
      title: 'Export OpenSCAD handoff bundle',
      description:
        'Export a SHA-256 verified openscad-viewer/handoff-bundle-v1 JSON package containing source, source-derived @language/@requires routing, and optional parameter presets.',
      inputSchema: selectorSchema({
        name: nameSchema.optional(),
        parameter_presets: parameterPresetsSchema.optional(),
      }),
      outputSchema: toolOutputSchema(z.object({ bundle: handoffBundleSchema })),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
    },
    async input => {
      try {
        let id: string | null = null
        let name = input.name ?? 'untitled.scad'
        let source: string
        let revision: number | null = null
        let createdAt: string | undefined

        if (input.model_id !== undefined) {
          const model = await store.getModel(input.model_id)
          if (!model) return errorResult(new ModelNotFoundError(input.model_id, input.revision))
          id = model.id
          name = input.name ?? model.name
          createdAt = model.updatedAt
          if (input.revision !== undefined) {
            const rev = await store.getModelRevision(input.model_id, input.revision)
            if (!rev) return errorResult(new ModelNotFoundError(input.model_id, input.revision))
            source = rev.source
            revision = rev.revision
            createdAt = rev.createdAt
          } else {
            source = model.source
            revision = model.revision
          }
        } else {
          source = input.source ?? ''
        }

        geometry.planSource(source, 'preview', 'preview')
        const bundle = createOpenScadHandoffBundle({
          origin: 'mcp-catalog',
          id,
          name,
          source,
          revision,
          createdAt,
          parameterPresets: input.parameter_presets,
        })
        return textResult({ bundle })
      } catch (error) {
        return errorResult(error)
      }
    },
  )

  server.registerTool(
    'openscad_import_handoff_bundle',
    {
      title: 'Import OpenSCAD handoff bundle',
      description:
        'Verify a browser/MCP openscad-viewer/handoff-bundle-v1 JSON package (SHA-256 digests and source-derived routing) and save into DuckDB only when strict CAS expected_revision matches (or idempotently unchanged).',
      inputSchema: z
        .object({
          bundle: looseHandoffBundleInputSchema,
          model_id: idSchema.optional(),
          name: nameSchema.optional(),
          expected_revision: z.number().int().nonnegative().optional(),
        })
        .strict(),
      outputSchema: toolOutputSchema(
        z.object({
          action: z.enum(['created', 'updated', 'unchanged']),
          model: modelMetadataSchema,
          bundle: handoffBundleSchema,
          source_routing: handoffRoutingSchema,
        }),
      ),
      annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false },
    },
    async input => {
      try {
        const verified = verifyOpenScadHandoffBundle(input.bundle)
        geometry.planSource(verified.document.source, 'preview', 'preview')
        const targetId = input.model_id ?? verified.document.id ?? undefined
        const targetName = input.name ?? verified.document.name

        if (targetId !== undefined) {
          const existing = await store.getModel(targetId)
          if (existing) {
            const sameContent = existing.source === verified.document.source && existing.name === targetName
            if (
              sameContent &&
              (input.expected_revision === undefined || input.expected_revision === existing.revision)
            ) {
              return textResult({
                action: 'unchanged' as const,
                model: modelToWire(existing),
                bundle: verified,
                source_routing: verified.routing,
              })
            }
            if (input.expected_revision === undefined) {
              throw new HandoffConflictError(
                `Model "${targetId}" exists at revision ${existing.revision}; supply expected_revision=${existing.revision} to apply handoff`,
                targetId,
                null,
                existing.revision,
              )
            }
            const updated = await store.saveModel({
              id: targetId,
              name: targetName,
              source: verified.document.source,
              expectedRevision: input.expected_revision,
            })
            notifyResourceListChanged()
            return textResult({
              action: 'updated' as const,
              model: modelToWire(updated),
              bundle: verified,
              source_routing: verified.routing,
            })
          }
        }

        const created = await store.saveModel({
          id: targetId,
          name: targetName,
          source: verified.document.source,
          expectedRevision: input.expected_revision,
        })
        notifyResourceListChanged()
        return textResult({
          action: 'created' as const,
          model: modelToWire(created),
          bundle: verified,
          source_routing: verified.routing,
        })
      } catch (error) {
        return errorResult(error)
      }
    },
  )
}
