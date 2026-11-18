import { Buffer } from 'node:buffer'
import { createHash } from 'node:crypto'
import { CURRENT_GEOMETRY_MANIFEST_VERSIONS } from '../../src/core/geometryExecution'
import { createOpenScadHandoffBundle } from '../../src/core/handoffBundle'
import { emptyDirectDocument, serializeDirectDocument } from '../../src/app/directModeling'
import { exportDirectStepV9 } from '../../src/kernel/cadNurbsStep'
import { createBrepBox } from '../../src/kernel/geometry/brep'
import { CAE_GUIDE } from '../../src/mcp/caeGuide'
import { compileRushGraph, RUSH_GRAPH_ASSEMBLY_EXAMPLE, RUSH_GRAPH_UNITS_EXAMPLE } from '../../src/language/rushGraph'
import { RUSH_GRAPH_NURBS_EXAMPLE, RUSH_GRAPH_NURBS_SURFACE_EXAMPLE } from '../../src/language/rushGraphNurbs'
import type {
  OfficialOpenScadCapabilities,
  OfficialOpenScadCheckResult,
  OfficialOpenScadExportResult,
  OfficialOpenScadRuntimeService,
} from '../../src/mcp/officialOpenScadRuntimeService'
import {
  OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES,
  OFFICIAL_OPENSCAD_EXPORT_FORMATS,
  OFFICIAL_OPENSCAD_MAX_LOG_BYTES,
  OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES,
  OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_FILES,
  OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES,
  OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS,
  OFFICIAL_OPENSCAD_MAX_WIRE_REQUEST_BYTES,
} from '../../src/mcp/officialOpenScadRuntimeProtocol'
import {
  OFFICIAL_OPENSCAD_FONT_FAMILY,
  OFFICIAL_OPENSCAD_FONT_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
  OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
  OFFICIAL_OPENSCAD_RUNTIME_VERSION,
} from '../../src/mcp/officialOpenScadRuntimePatch'

export interface ToolMatrixFixture {
  readonly validArgs: Record<string, unknown>
  readonly invalidArgs: Record<string, unknown>
}

export interface CoverageSeedIds {
  readonly buildId: string
  readonly artifactId: string
  readonly independentArtifactUri: string
  readonly officialArtifactUri: string
  readonly closedEditorPort: number
}

const sha = (value: string | Uint8Array) => createHash('sha256').update(value).digest('hex')

export function createMockOfficialRuntime(): OfficialOpenScadRuntimeService {
  const exported = new Uint8Array([1, 2, 3, 4])
  const capabilities: OfficialOpenScadCapabilities = {
    available: true,
    unavailableReason: null,
    expectedRuntimeVersion: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
    runtimeVersion: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
    archiveSha256: OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
    runtimeSha256: '1'.repeat(64),
    patchVersion: OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
    defaultFont: {
      family: OFFICIAL_OPENSCAD_FONT_FAMILY,
      filename: OFFICIAL_OPENSCAD_FONT_FILENAME,
      sha256: '2'.repeat(64),
      licenseFilename: OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
      licenseSha256: '3'.repeat(64),
    },
    isolation: {
      filesystem: 'MEMFS',
      nodePermissionModel: true,
      scadHostFileRead: false,
      scadHostFileWrite: false,
      networkApiExposed: false,
      networkSandboxEnforced: false,
      wasmMemoryLimitEnforced: false,
      processModel: 'one-shot',
    },
    formats: OFFICIAL_OPENSCAD_EXPORT_FORMATS,
    experimentalFeatures: OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES,
    defaults: {
      backend: 'Manifold',
      hardWarnings: true,
      checkParameters: true,
      checkParameterRanges: true,
      experimentsEnabled: false,
    },
    limits: {
      sourceBytes: OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES,
      projectFiles: OFFICIAL_OPENSCAD_MAX_PROJECT_FILES,
      projectFileBytes: OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
      projectBytes: OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES,
      wireRequestBytes: OFFICIAL_OPENSCAD_MAX_WIRE_REQUEST_BYTES,
      outputBytes: OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES,
      logBytes: OFFICIAL_OPENSCAD_MAX_LOG_BYTES,
      logEntries: OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES,
      timeoutMs: OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS,
    },
    setupCommand: 'npm run setup:openscad',
  }
  return {
    capabilities: async () => capabilities,
    check: async (source, options): Promise<OfficialOpenScadCheckResult> => ({
      runtimeVersion: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
      sourceSha256: sha(source),
      csgSha256: sha('csg'),
      csgBytes: 8,
      durationMs: 2,
      logs: { stdout: [], stderr: [], truncated: false },
      experimentalFeatures: options?.experimentalFeatures ?? [],
    }),
    export: async (source, format, options): Promise<OfficialOpenScadExportResult> => ({
      runtimeVersion: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
      sourceSha256: sha(source),
      format,
      mimeType: format === 'stl' ? 'model/stl' : 'application/octet-stream',
      fileName: `model.${format}`,
      data: exported,
      sha256: sha(exported),
      durationMs: 4,
      logs: { stdout: [], stderr: [], truncated: false },
      experimentalFeatures: options?.experimentalFeatures ?? [],
    }),
    close: async () => {},
  }
}

const TET_OBJ = Buffer.from(
  ['v 0 0 0', 'v 10 0 0', 'v 0 10 0', 'v 0 0 10', 'f 1 3 2', 'f 1 2 4', 'f 2 3 4', 'f 3 1 4'].join('\n'),
  'utf8',
).toString('base64')

const SQUARE_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10"><rect x="1" y="1" width="8" height="8"/></svg>'

const OFFSET_DOC = {
  language: 'rush/nurbs-1' as const,
  units: 'mm' as const,
  parameters: [],
  nodes: [
    { id: 'a', op: 'plane_patch', origin: [0, 0, 0], axis_u: [1, 0, 0], axis_v: [0, 1, 0] },
    {
      id: 'b',
      op: 'transform',
      input: 'a',
      matrix: [
        [1, 0, 0, 0],
        [0, 1, 0, 0],
        [0, 0, 1, 0.25],
        [0, 0, 0, 1],
      ],
    },
  ],
  root: 'b',
}

const BREP_BOX_DOC = (min: [number, number, number], max: [number, number, number]) => ({
  language: 'rush/nurbs-1',
  units: 'mm',
  parameters: [],
  nodes: [{ id: 'b', op: 'brep_box', min, max }],
  root: 'b',
})

const XY_PATCH = {
  degreeU: 1,
  degreeV: 1,
  knotsU: [0, 0, 1, 1],
  knotsV: [0, 0, 1, 1],
  controlPoints: [
    [
      [0, 0, 0],
      [0, 1, 0],
    ],
    [
      [1, 0, 0],
      [1, 1, 0],
    ],
  ],
  weights: [
    [1, 1],
    [1, 1],
  ],
}

const XZ_PATCH = {
  ...XY_PATCH,
  controlPoints: [
    [
      [0, 0, 0],
      [0, 0, 1],
    ],
    [
      [1, 0, 0],
      [1, 0, 1],
    ],
  ],
}

export function buildToolFixtures(seed: CoverageSeedIds): Record<string, ToolMatrixFixture> {
  const rushCompiled = compileRushGraph(RUSH_GRAPH_UNITS_EXAMPLE)
  const boxStep = exportDirectStepV9(createBrepBox([0, 0, 0], [4, 4, 4])).text
  const emptyDoc = serializeDirectDocument(emptyDirectDocument())
  const directBoxScript = JSON.stringify({
    version: 1,
    actions: [
      {
        kind: 'primitive',
        options: {
          kind: 'box',
          id: 'bx',
          name: 'Box',
          size: 4,
          topRadius: 1,
          innerRadius: 0.5,
          round: 'exact',
          segments: 1,
        },
      },
    ],
  })
  const invalidTarget = { target: { source: '' } }
  const invalidDoc = { document: { language: 'bad' } }

  return {
    openscad_list_models: { validArgs: {}, invalidArgs: { limit: -1 } },
    openscad_get_model: { validArgs: { id: 'seed:part' }, invalidArgs: { id: 'missing:none' } },
    openscad_save_model: {
      validArgs: { id: 'matrix:saved', name: 'Saved', source: 'cube(3);' },
      invalidArgs: { id: '' },
    },
    openscad_delete_model: { validArgs: { model_id: 'seed:disposable' }, invalidArgs: { model_id: '' } },
    openscad_catalog_stats: { validArgs: {}, invalidArgs: { extra: 1 } },
    openscad_list_engines: { validArgs: {}, invalidArgs: { extra: 1 } },
    openscad_brep_capabilities: { validArgs: {}, invalidArgs: { extra: 1 } },
    openscad_official_status: { validArgs: {}, invalidArgs: { extra: 1 } },
    openscad_official_check: { validArgs: { source: 'cube(2);' }, invalidArgs: { source: 123 } },
    openscad_official_export: {
      validArgs: { source: 'cube(2);', format: 'stl' },
      invalidArgs: { source: 'cube(2);', format: 'invalid' },
    },
    openscad_independent_check: {
      validArgs: { source: 'cube(2);', quality: 'full' },
      invalidArgs: { source: 'bad(;' },
    },
    openscad_independent_export: {
      validArgs: { source: 'cube(2);', format: 'stl' },
      invalidArgs: { source: 'bad(;', format: 'stl' },
    },
    openscad_list_model_revisions: { validArgs: { id: 'seed:part' }, invalidArgs: { id: '' } },
    openscad_check: { validArgs: { source: 'cube(2);', quality: 'full' }, invalidArgs: { source: 'bad(;' } },
    openscad_compare: {
      validArgs: { left: { source: 'cube(2);' }, right: { source: 'cube(3);' }, quality: 'full' },
      invalidArgs: { left: { source: 'bad(;' }, right: { source: 'cube(2);' } },
    },
    openscad_analyze: { validArgs: { model_id: 'seed:part', quality: 'full' }, invalidArgs: { source: 'bad(;' } },
    openscad_customize: {
      validArgs: { source: 'w = 4;\ncube([w, 4, 4]);', values: { w: 6 } },
      invalidArgs: { source: 'w = 4;\ncube([w,4,4]);', values: { missing: 1 } },
    },
    openscad_customize_model: {
      validArgs: { model_id: 'seed:customizable', expected_revision: 0, values: { w: 8 } },
      invalidArgs: { model_id: 'seed:customizable', expected_revision: 99, values: { w: 8 } },
    },
    openscad_export: {
      validArgs: { model_id: 'seed:part', format: 'stl' },
      invalidArgs: { source: 'bad(;', format: 'stl' },
    },
    openscad_build_history: { validArgs: { limit: 10 }, invalidArgs: { limit: 0 } },
    openscad_parameter_sweep: {
      validArgs: { source: 'w = 4;\ncube([w, 4, 4]);', axes: [{ name: 'w', values: [4, 6] }] },
      invalidArgs: { source: 'cube(2);', axes: [] },
    },
    openscad_editor_status: { validArgs: { port: seed.closedEditorPort }, invalidArgs: { port: 0 } },
    openscad_open_in_editor: {
      validArgs: { model_id: 'seed:part', port: seed.closedEditorPort },
      invalidArgs: { model_id: 'missing:none', port: seed.closedEditorPort },
    },
    openscad_export_handoff_bundle: { validArgs: { model_id: 'seed:part' }, invalidArgs: { model_id: 'missing:none' } },
    openscad_import_handoff_bundle: {
      validArgs: {
        bundle: createOpenScadHandoffBundle({
          origin: 'browser-workspace',
          id: 'matrix:handoff',
          name: 'Handoff',
          source: 'cube(5);',
          revision: null,
        }),
      },
      invalidArgs: {
        bundle: { schema: 'wrong', origin: 'x', created_at: 'y', document: {}, routing: {}, bundle_sha256: '0' },
      },
    },
    rush_language: { validArgs: {}, invalidArgs: { extra: 1 } },
    rush_frontend_compile: {
      validArgs: { source: '// @rush/1\nshow box([2mm,2mm,2mm])' },
      invalidArgs: { source: 'invalid syntax @@@' },
    },
    rush_compile: { validArgs: { document: RUSH_GRAPH_UNITS_EXAMPLE }, invalidArgs: invalidDoc },
    rush_check: { validArgs: { document: RUSH_GRAPH_UNITS_EXAMPLE }, invalidArgs: invalidDoc },
    rush_set_parameters: {
      validArgs: {
        document: RUSH_GRAPH_UNITS_EXAMPLE,
        expected_document_sha256: rushCompiled.document_sha256,
        updates: [{ id: 'width', value: 5 }],
      },
      invalidArgs: {
        document: RUSH_GRAPH_UNITS_EXAMPLE,
        expected_document_sha256: '0'.repeat(64),
        updates: [{ id: 'width', value: 5 }],
      },
    },
    rush_svg_preview: { validArgs: { svg: SQUARE_SVG }, invalidArgs: { svg: '<script/>' } },
    rush_svg_extrude: { validArgs: { svg: SQUARE_SVG, height: 3 }, invalidArgs: { svg: '<svg/>', height: 3 } },
    rush_svg_export: { validArgs: { document: RUSH_GRAPH_UNITS_EXAMPLE, axis: 'z' }, invalidArgs: invalidDoc },
    rush_generate: {
      validArgs: { kind: 'thread', length: 6, internal: false },
      invalidArgs: { kind: 'thread', length: -1 },
    },
    rush_modify: {
      validArgs: {
        document: RUSH_GRAPH_UNITS_EXAMPLE,
        modification: { operation: 'resize', size: [10, 10, 10], anchor: 'minimum' },
      },
      invalidArgs: invalidDoc,
    },
    rush_export: {
      validArgs: { document: RUSH_GRAPH_UNITS_EXAMPLE, format: 'stl' },
      invalidArgs: { document: RUSH_GRAPH_UNITS_EXAMPLE, format: 'bad' },
    },
    mesh_convert: {
      validArgs: { file_name: 'tet.obj', data_base64: TET_OBJ, format: 'stl' },
      invalidArgs: { file_name: 'tet.obj', data_base64: '!!!', format: 'stl' },
    },
    rush_nurbs_language: { validArgs: {}, invalidArgs: { extra: 1 } },
    rush_nurbs_compile: { validArgs: { document: RUSH_GRAPH_NURBS_SURFACE_EXAMPLE }, invalidArgs: invalidDoc },
    rush_nurbs_offset_deviation: {
      validArgs: {
        document: OFFSET_DOC,
        source: 'a',
        candidate: 'b',
        distance: 0.25,
        tolerance: 1e-8,
        maxCells: 1000,
        maxDepth: 24,
      },
      invalidArgs: {
        document: OFFSET_DOC,
        source: 'missing',
        candidate: 'b',
        distance: 0.25,
        tolerance: 1e-8,
        maxCells: 1000,
        maxDepth: 24,
      },
    },
    rush_nurbs_evaluate: {
      validArgs: { document: RUSH_GRAPH_NURBS_EXAMPLE, evaluations: [{ node: 'arc', u: 0.5 }] },
      invalidArgs: { document: RUSH_GRAPH_NURBS_EXAMPLE, evaluations: [] },
    },
    rush_nurbs_build: { validArgs: { document: RUSH_GRAPH_NURBS_SURFACE_EXAMPLE }, invalidArgs: invalidDoc },
    rush_nurbs_export: {
      validArgs: { document: RUSH_GRAPH_NURBS_SURFACE_EXAMPLE, format: 'stl' },
      invalidArgs: { document: RUSH_GRAPH_NURBS_EXAMPLE, format: 'step' },
    },
    rush_nurbs_intersect: {
      validArgs: { mode: 'surface_surface', first: XY_PATCH, second: XZ_PATCH },
      invalidArgs: { mode: 'surface_surface', first: {}, second: {} },
    },
    rush_report: { validArgs: { document: RUSH_GRAPH_UNITS_EXAMPLE }, invalidArgs: invalidDoc },
    rush_interference: { validArgs: { document: RUSH_GRAPH_ASSEMBLY_EXAMPLE }, invalidArgs: invalidDoc },
    openscad_direct_transaction: {
      validArgs: { document: emptyDoc, script: directBoxScript },
      invalidArgs: { document: emptyDoc, script: '{}' },
    },
    direct_create_document: { validArgs: { units: 'mm' }, invalidArgs: { units: 'lightyear' } },
    direct_inspect_document: { validArgs: { document: emptyDoc }, invalidArgs: { document: '{bad' } },
    sketch_solve: {
      validArgs: {
        points: [
          { id: 'p0', position: [0, 0] },
          { id: 'p1', position: [4, 0.1] },
          { id: 'p2', position: [0.1, 3] },
        ],
        constraints: [
          { id: 'f', kind: 'fix', point: 'p0', at: [0, 0] },
          { id: 'h', kind: 'horizontal', a: 'p0', b: 'p1' },
          { id: 'd', kind: 'distance', a: 'p0', b: 'p1', value: 5 },
        ],
      },
      invalidArgs: { points: [], constraints: [] },
    },
    brep_inspect_contacts: {
      validArgs: { model: { nurbs_document: BREP_BOX_DOC([0, 0, 0], [2, 2, 2]) } },
      invalidArgs: { model: { step_text: 'BAD_STEP_HEADER' } },
    },
    brep_solid_distance: {
      validArgs: {
        a: { nurbs_document: BREP_BOX_DOC([0, 0, 0], [2, 2, 2]) },
        b: { nurbs_document: BREP_BOX_DOC([5, 0, 0], [7, 2, 2]) },
      },
      invalidArgs: { a: { step_text: 'BAD_STEP_HEADER' }, b: { step_text: 'BAD_STEP_HEADER' } },
    },
    cad_import_step: { validArgs: { step_text: boxStep }, invalidArgs: { step_text: 'INVALID_STEP_TEXT' } },
    cad_export_iges: {
      validArgs: { model: { nurbs_document: BREP_BOX_DOC([0, 0, 0], [2, 2, 2]) } },
      invalidArgs: { model: { iges_text: 'INVALID_IGES_TEXT' } },
    },
    cad_import_iges: {
      validArgs: {
        iges_text: (() => {
          throw new Error('replaced dynamically')
        }) as never,
      },
      invalidArgs: { iges_text: 'INVALID_IGES_TEXT' },
    },
    mesh_inspect: { validArgs: { target: { source: 'cube(4);' } }, invalidArgs: invalidTarget },
    mesh_repair: { validArgs: { target: { source: 'cube(4);' }, mode: 'conservative' }, invalidArgs: invalidTarget },
    cad_drawing_sheet: {
      validArgs: { target: { source: 'cube(4);' }, include_pdf: false },
      invalidArgs: invalidTarget,
    },
    cad_iso286_fit: {
      validArgs: { nominal_diameter_mm: 20, hole_zone: 'H7', shaft_zone: 'g6' },
      invalidArgs: { nominal_diameter_mm: -1, hole_zone: 'H7', shaft_zone: 'g6' },
    },
    cam_print_readiness: { validArgs: { target: { source: 'cube(4);' } }, invalidArgs: invalidTarget },
    cam_slice_gcode: { validArgs: { target: { source: 'cube(4);' }, mode: 'preview' }, invalidArgs: invalidTarget },
    cam_inspect_gcode: {
      validArgs: { gcode: 'G90\nG21\nG0 X0 Y0 Z0.2 F3000\nG1 X10 Y0 Z0.2 E1 F1200\n' },
      invalidArgs: { gcode: '' },
    },
    cam_laser_section_plan: {
      validArgs: { target: { source: 'cube([10,10,4]);' }, z_mm: 2, laser_mode_confirmed: true },
      invalidArgs: { target: { source: 'cube([10,10,4]);' }, z_mm: 999 },
    },
    cae_truss_analyze: {
      validArgs: CAE_GUIDE.examples.truss,
      invalidArgs: { nodes_mm: [], members: [], restrained: [], forces_n: [] },
    },
    cae_frame_analyze: {
      validArgs: CAE_GUIDE.examples.frame,
      invalidArgs: { nodes_mm: [], members: [], restrained: [], forces_n: [], moments_nmm: [] },
    },
    cae_printed_solid_analyze: { validArgs: CAE_GUIDE.examples.printed_solid, invalidArgs: { nodes_mm: [] } },
  }
}

export function buildTemplateUris(seed: CoverageSeedIds): Record<string, string> {
  return {
    'openscad://engines/{engine_class}/capabilities/{manifest_version}': `openscad://engines/mesh/capabilities/${CURRENT_GEOMETRY_MANIFEST_VERSIONS.mesh}`,
    'openscad://models/{id}': 'openscad://models/seed%3Apart',
    'openscad://models/{id}/revisions/{revision}': 'openscad://models/seed%3Apart/revisions/0',
    'openscad://builds/{id}': `openscad://builds/${encodeURIComponent(seed.buildId)}`,
    'openscad://artifacts/{id}': `openscad://artifacts/${encodeURIComponent(seed.artifactId)}`,
    'openscad://independent-artifacts/{sha256}': seed.independentArtifactUri,
    'openscad://official-artifacts/{id}': seed.officialArtifactUri,
    'openscad://examples/{name}': 'openscad://examples/basic',
  }
}

export const PROMPT_ARGUMENTS: Record<string, Record<string, string>> = {
  openscad_review_model: { model_id: 'seed:part', goal: 'Verify dimensions' },
  openscad_customize_workflow: { model_id: 'seed:part', goal: 'Widen box to 6mm' },
  cad_step_iges_roundtrip_audit: { format: 'step', goal: 'Check AP242 faces' },
  rush_assembly_clearance_review: { min_clearance_mm: '0.5', goal: 'Check interference' },
  direct_brep_modeling_workflow: { goal: 'Create enclosure' },
  cam_print_and_slice_workflow: { layer_height_mm: '0.2', goal: 'Prepare FDM print' },
  cae_structural_optimization: { target_safety_factor: '2.0', goal: 'Verify cantilever stress' },
}
