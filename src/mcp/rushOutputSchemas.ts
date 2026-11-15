import { z } from 'zod/v4'
import { MESH_EXPORT_FORMATS } from '../exchange/meshExportFormats'
import { MESH_IMPORT_FORMATS } from '../exchange/meshConvert'

const sha256Schema = z.string().regex(/^[a-f0-9]{64}$/)
const vector3Schema = z.tuple([z.number(), z.number(), z.number()])
const boundsSchema = z.object({ min: vector3Schema, max: vector3Schema }).nullable()
const nurbsBoundsSchema = z
  .object({
    min: z.array(z.number()).min(2).max(3),
    max: z.array(z.number()).min(2).max(3),
  })
  .nullable()

const geometryCheckResultSchema = z.looseObject({
  id: z.string(),
  check: z.string(),
  expected: z.number(),
  tolerance: z.number(),
  message: z.string(),
  actual: z.number().nullable(),
  status: z.enum(['passed', 'failed', 'unknown']),
})

export const rushErrorResultSchema = z.looseObject({
  error: z.looseObject({
    code: z.string(),
    message: z.string(),
    path: z.string().optional(),
  }),
})

export function rushToolOutputSchema<T extends z.ZodType>(success: T) {
  return z.union([success, rushErrorResultSchema])
}

const headlessAnalysisSchema = z.looseObject({
  execution: z.looseObject({
    languageContract: z.string(),
    engineClass: z.string(),
    engineKey: z.string(),
    quality: z.enum(['preview', 'full']),
    purpose: z.enum(['preview', 'full', 'analysis', 'export']),
    automaticFallback: z.literal(false),
  }),
  quality: z.enum(['preview', 'full']),
  reduced: z.boolean(),
  durationMs: z.number().nonnegative(),
  meshCount: z.number().int().nonnegative(),
  vertexCount: z.number().int().nonnegative(),
  triangleCount: z.number().int().nonnegative(),
  volume: z.number(),
  surfaceArea: z.number(),
  bounds: boundsSchema,
  topology: z.object({
    boundary: z.number().int().nonnegative(),
    crease: z.number().int().nonnegative(),
    nonManifold: z.number().int().nonnegative(),
    degenerate: z.number().int().nonnegative(),
  }),
  warnings: z.array(z.string()),
})

const rushIrDocumentSummarySchema = z.looseObject({
  language: z.literal('rush/ir-1'),
  units: z.string(),
  parameters: z.array(z.unknown()),
  nodes: z.array(z.unknown()),
  root: z.string(),
})

const rushIrCompilationBaseSchema = z.looseObject({
  document: rushIrDocumentSummarySchema,
  geometry_assertions: z.array(z.looseObject({ id: z.string(), target: z.string(), check: z.string() })),
  constraint_report: z.array(
    z.looseObject({ id: z.string(), passed: z.boolean(), status: z.enum(['passed', 'failed']) }),
  ),
  sketch_solutions: z.array(z.unknown()),
  assembly_components: z.array(z.unknown()),
  mechanical_reports: z.array(z.record(z.string(), z.unknown())),
  mechanical_parts: z.array(z.record(z.string(), z.unknown())),
  document_sha256: sha256Schema,
  source: z.string(),
  source_map: z.array(z.looseObject({ node_id: z.string(), line: z.number(), instance_path: z.string() })),
  execution_target: z.literal('legacy/current+own-rust-cad'),
})

export const meshConvertOutputSchema = rushToolOutputSchema(
  z.object({
    format: z.enum(MESH_EXPORT_FORMATS),
    file_name: z.string(),
    mime_type: z.string(),
    byte_length: z.number().int().nonnegative(),
    source: z.object({
      format: z.enum(MESH_IMPORT_FORMATS),
      byte_length: z.number().int().nonnegative(),
      triangle_count: z.number().int().nonnegative(),
      vertex_count: z.number().int().nonnegative(),
      source_vertex_count: z.number().int().nonnegative(),
      degenerate_triangles: z.number().int().nonnegative(),
    }),
  }),
)

export const rushLanguageOutputSchema = rushToolOutputSchema(
  z.looseObject({
    text_guide: z.string(),
    language: z.literal('rush/ir-1'),
    mechanical_examples: z.unknown(),
    guide: z.string(),
    sketch_example: z.unknown(),
    assembly_example: z.unknown(),
    loft_example: z.unknown(),
    units_example: z.unknown(),
    functional_example: z.unknown(),
    schema: z.record(z.string(), z.unknown()),
    example: z.unknown(),
    instructions: z.string(),
  }),
)

const rushFrontendNurbsCompiledSchema = z.looseObject({
  document: z.looseObject({
    language: z.literal('rush/nurbs-1'),
    units: z.string(),
    nodes: z.array(z.unknown()),
    root: z.string(),
  }),
  resolved_document: z.unknown(),
  document_sha256: sha256Schema,
  execution_target: z.literal('own-nurbs'),
  customizer: z.array(z.unknown()),
})

export const rushFrontendCompileOutputSchema = rushToolOutputSchema(
  z.union([rushIrCompilationBaseSchema.extend({ customizer: z.array(z.unknown()) }), rushFrontendNurbsCompiledSchema]),
)

export const rushCompileOutputSchema = rushToolOutputSchema(rushIrCompilationBaseSchema)
export const rushSetParametersOutputSchema = rushToolOutputSchema(rushIrCompilationBaseSchema)

export const rushCheckOutputSchema = rushToolOutputSchema(
  rushIrCompilationBaseSchema.extend({
    analysis: headlessAnalysisSchema,
    checks: z.array(geometryCheckResultSchema),
  }),
)

export const rushGenerateOutputSchema = rushToolOutputSchema(
  rushIrCompilationBaseSchema.extend({
    analysis: headlessAnalysisSchema,
    images_status: z.enum(['rendered', 'unavailable']),
    image_error: z.string().nullable(),
    preview_document_sha256: sha256Schema,
    views: z.array(z.string()),
    printability: z.literal('unknown'),
  }),
)

export const rushModifyOutputSchema = rushToolOutputSchema(
  z.object({
    operation: z.enum(['resize', 'split']),
    measurement_document_sha256: sha256Schema,
    results: z.array(
      rushIrCompilationBaseSchema.extend({
        analysis: headlessAnalysisSchema,
        side: z.string().optional(),
      }),
    ),
    note: z.string(),
  }),
)

export const rushExportOutputSchema = rushToolOutputSchema(
  z.object({
    document_sha256: sha256Schema,
    format: z.enum(MESH_EXPORT_FORMATS),
    mime_type: z.string(),
    extension: z.string(),
    byte_length: z.number().int().nonnegative(),
  }),
)

export const rushReportOutputSchema = rushToolOutputSchema(
  z.looseObject({
    checks: z.array(geometryCheckResultSchema),
    document_sha256: sha256Schema,
    analysis: headlessAnalysisSchema,
    constraints: z.array(z.unknown()),
    sketch_solutions: z.array(z.unknown()),
    assembly_components: z.array(z.unknown()),
    mechanical_reports: z.array(z.record(z.string(), z.unknown())),
    mechanical_parts: z.array(z.record(z.string(), z.unknown())),
    references: z.array(
      z.object({
        mesh: z.number().int().nonnegative(),
        triangle_start: z.number().int().nonnegative(),
        triangle_end: z.number().int().nonnegative(),
        node_id: z.string().nullable(),
        instance_path: z.string().nullable(),
      }),
    ),
    references_truncated: z.boolean(),
    reference_scope: z.string(),
    images_status: z.enum(['rendered', 'unavailable']),
    image_error: z.string().nullable(),
    views: z.array(z.string()),
    printability: z.literal('unknown'),
    markdown_summary: z.string(),
  }),
)

export const rushInterferenceOutputSchema = rushToolOutputSchema(
  z.object({
    document_sha256: sha256Schema,
    scope: z.string(),
    volume_tolerance_mm3: z.number().positive(),
    pairs: z.array(
      z.object({
        a: z.string(),
        b: z.string(),
        status: z.enum(['overlap', 'no_volume_overlap', 'unknown']),
        intersection_volume_mm3: z.number().nullable(),
        method: z.string(),
      }),
    ),
    status: z.enum(['overlap', 'no_volume_overlap', 'unknown']),
    markdown_report: z.string(),
  }),
)

export const rushSvgPreviewOutputSchema = rushToolOutputSchema(
  z.object({
    widthMm: z.number().nonnegative(),
    heightMm: z.number().nonnegative(),
    warnings: z.array(z.string()),
  }),
)

export const rushSvgExtrudeOutputSchema = rushToolOutputSchema(
  z.object({
    source: z.string(),
    svg: z.string(),
    warnings: z.array(z.string()),
    analysis: headlessAnalysisSchema,
    document: rushIrDocumentSummarySchema.optional(),
    document_sha256: sha256Schema.optional(),
    rush_source: z.string().optional(),
    rush_analysis: headlessAnalysisSchema.optional(),
  }),
)

export const rushSvgExportOutputSchema = rushToolOutputSchema(
  z.object({
    document_sha256: sha256Schema,
    axis: z.enum(['x', 'y', 'z']),
    mime_type: z.literal('image/svg+xml'),
    byte_length: z.number().int().nonnegative(),
  }),
)

const nurbsReportSchema = z.looseObject({
  bounds_scope: z.string(),
  kernel: z.string(),
  polygon_core: z.string(),
  geometry_authority: z.string(),
  root: z.string(),
  root_kind: z.string(),
  bounds: nurbsBoundsSchema,
  definitions: z.record(z.string(), z.unknown()),
  mesh: z.unknown(),
  printability: z.literal('unknown'),
  error_bound_certified: z.boolean(),
})

const nurbsBaseResultSchema = z.looseObject({
  ok: z.literal(true),
  document_sha256: sha256Schema,
  execution_target: z.literal('own-nurbs'),
  automatic_fallback: z.literal(false),
  report: nurbsReportSchema,
  evaluations: z.array(z.record(z.string(), z.unknown())),
})

export const rushNurbsLanguageOutputSchema = rushToolOutputSchema(
  z.looseObject({
    language: z.literal('rush/nurbs-1'),
    guide: z.string(),
    schema: z.record(z.string(), z.unknown()),
    example: z.unknown(),
    surface_example: z.unknown(),
  }),
)

export const rushNurbsCompileOutputSchema = rushToolOutputSchema(
  z.looseObject({
    document: z.unknown(),
    resolved_document: z.unknown(),
    document_sha256: sha256Schema,
    execution_target: z.literal('own-nurbs'),
  }),
)

export const rushNurbsOffsetDeviationOutputSchema = rushToolOutputSchema(nurbsBaseResultSchema)
export const rushNurbsEvaluateOutputSchema = rushToolOutputSchema(nurbsBaseResultSchema)
export const rushNurbsBuildOutputSchema = rushToolOutputSchema(
  nurbsBaseResultSchema.extend({
    images_status: z.enum(['rendered', 'unavailable']),
    image_error: z.string().nullable(),
    views: z.array(z.string()),
  }),
)
export const rushNurbsExportOutputSchema = rushToolOutputSchema(nurbsBaseResultSchema)

export const rushNurbsIntersectOutputSchema = rushToolOutputSchema(
  z.union([
    z.looseObject({
      version: z.literal('nurbs-foundation/5'),
      kind: z.enum(['curve_curve', 'curve_surface']),
      coverage: z.record(z.string(), z.unknown()),
      components: z.array(z.record(z.string(), z.unknown())),
      unresolved: z.array(z.record(z.string(), z.unknown())),
      rounding: z.enum(['binary64-nextafter-outward', 'uncertified-binary64']),
      evidence: z.record(z.string(), z.unknown()),
    }),
    z.looseObject({
      version: z.literal('nurbs-ss/1'),
      kind: z.literal('surface_surface'),
      coverage: z.record(z.string(), z.unknown()),
      components: z.array(z.record(z.string(), z.unknown())),
      branchGraph: z.record(z.string(), z.unknown()),
      uvArrangement: z.record(z.string(), z.unknown()),
      unresolved: z.array(z.record(z.string(), z.unknown())),
      rounding: z.enum(['original_source_and_checked_binary64', 'candidate_binary64', 'binary64-nextafter-outward']),
      evidence: z.record(z.string(), z.unknown()),
      topologyAuthority: z.record(z.string(), z.unknown()),
      booleanMutationAuthority: z.literal(false),
    }),
  ]),
)
