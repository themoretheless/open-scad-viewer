import { afterEach, describe, expect, it, vi } from 'vitest'
import { z } from 'zod/v4'
import { InMemoryTransport, type JSONRPCMessage } from '@modelcontextprotocol/server'
import {
  createOpenScadHandoffBundle,
  exportWorkspaceHandoffBundle,
  HandoffBundleValidationError,
  HandoffConflictError,
  OPENSCAD_HANDOFF_BUNDLE_SCHEMA,
  prepareWorkspaceHandoffImport,
  verifyOpenScadHandoffBundle,
} from '../src/core/handoffBundle'
import { GeometryBuildEngine } from '../src/kernel/geometryBuildEngine'
import { RUSH_GRAPH_ASSEMBLY_EXAMPLE, RUSH_GRAPH_UNITS_EXAMPLE } from '../src/language/rushGraph'
import { BoundedTransport } from '../src/mcp/boundedTransport'
import { createRushGraphMcpServer as createOpenScadMcpServer } from '../src/mcp/createRushGraphServer'
import { DuckDbModelStore } from '../src/mcp/duckdbModelStore'
import { GeometryCompileCache } from '../src/mcp/geometryCompileCache'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import { SWEEP_RANK_OPTIONS } from '../src/mcp/parameterSweep'
import { meshConvertOutputSchema } from '../src/mcp/rushOutputSchemas'

interface JsonRpcResponse {
  jsonrpc: '2.0'
  id: number
  result?: Record<string, unknown>
  error?: { code: number; message: string }
}

const closeables: Array<() => Promise<void>> = []

afterEach(async () => {
  await Promise.all(closeables.splice(0).map(close => close()))
})

async function connectedServer(geometry?: HeadlessGeometryService) {
  const store = await DuckDbModelStore.open(':memory:')
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()
  const boundedTransport = new BoundedTransport(serverTransport, {
    maxInFlightRequests: 8,
    maxPendingOutboundMessages: 24,
  })
  const server = createOpenScadMcpServer({
    store,
    geometry,
    onRequestSettled: requestId => boundedTransport.settleRequest(requestId),
    onRequestCancelled: requestId => boundedTransport.cancelRequest(requestId),
  })
  const pending = new Map<number, (response: JsonRpcResponse) => void>()
  clientTransport.onmessage = message => {
    if ('id' in message && typeof message.id === 'number' && ('result' in message || 'error' in message)) {
      pending.get(message.id)?.(message as JsonRpcResponse)
      pending.delete(message.id)
    }
  }
  await clientTransport.start()
  await server.connect(boundedTransport)
  let nextId = 1

  const request = async (method: string, params: Record<string, unknown> = {}) => {
    const id = nextId++
    const response = new Promise<JsonRpcResponse>((resolve, reject) => {
      const timeout = setTimeout(() => {
        pending.delete(id)
        reject(new Error(`Timed out waiting for ${method}`))
      }, 10_000)
      pending.set(id, value => {
        clearTimeout(timeout)
        resolve(value)
      })
    })
    await clientTransport.send({ jsonrpc: '2.0', id, method, params } as JSONRPCMessage)
    const value = await response
    await new Promise<void>(resolve => queueMicrotask(resolve))
    if (value.error) throw new Error(`${value.error.code}: ${value.error.message}`)
    return value.result ?? {}
  }

  await request('initialize', {
    protocolVersion: '2025-11-25',
    capabilities: {},
    clientInfo: { name: 'next-stage-test', version: '1.0.0' },
  })
  await clientTransport.send({ jsonrpc: '2.0', method: 'notifications/initialized' } as JSONRPCMessage)

  closeables.push(async () => {
    await server.close()
    await store.close()
  })

  return { request, store }
}

describe('Next-stage MCP & B-rep STEP features', () => {
  it('caches compiled geometry by source+execution route, isolates buffers, evicts LRU, and bypasses unhealthy runtimes', async () => {
    const engine = new GeometryBuildEngine()
    await engine.initializeSource('cube(2);', { quality: 'preview', purpose: 'preview' })
    let healthy = true
    const build = vi.fn(
      (
        source: string,
        quality: 'preview' | 'full',
        purpose: 'preview' | 'full' | 'analysis' | 'export',
        signal?: AbortSignal,
      ) => engine.buildSource(source, { quality, purpose }, { shouldAbort: () => signal?.aborted ?? false }),
    )
    const runtime = {
      capabilities: vi.fn(() => engine.capabilities()),
      isHealthy: () => healthy,
      build,
    }
    const cache = new GeometryCompileCache({ maxEntries: 2, maxBytes: 10 * 1024 * 1024 })
    const service = new HeadlessGeometryService(engine, runtime, { compileCache: cache })

    const first = await service.compile('cube(2);', 'preview', undefined, 'preview')
    const second = await service.compile('cube(2);', 'preview', undefined, 'preview')
    expect(build).toHaveBeenCalledTimes(1)
    expect(service.compileCacheStats()).toMatchObject({ hits: 1, misses: 1, entries: 1 })

    // Mutating returned TypedArray must not corrupt subsequent cache hits.
    first.meshes[0]!.vertices[0] = 99999
    expect(second.meshes[0]!.vertices[0]).not.toBe(99999)
    const third = await service.compile('cube(2);', 'preview', undefined, 'preview')
    expect(third.meshes[0]!.vertices[0]).not.toBe(99999)

    // Evicts least-recently-used entry when capped at 2 entries.
    await service.compile('cube(3);', 'preview', undefined, 'preview')
    await service.compile('cube(4);', 'preview', undefined, 'preview')
    expect(service.compileCacheStats()).toMatchObject({ entries: 2, evictions: 1 })

    // Bypasses cache cleanly when runtime reports unhealthy.
    healthy = false
    await service.compile('cube(4);', 'preview', undefined, 'preview')
    expect(build).toHaveBeenCalledTimes(4)
  })

  it('runs bounded read-only openscad_parameter_sweep with deterministic ranking and zero catalog writes', async () => {
    const { request, store } = await connectedServer()
    const source = `w = 4; // [1:20]\nh = 2; // [1:20]\ncube([w, 3, h]);`
    const result = (await request('tools/call', {
      name: 'openscad_parameter_sweep',
      arguments: {
        source,
        axes: [
          { name: 'w', values: [2, 6, 4] },
          { name: 'h', values: [1, 3] },
        ],
        rank_by: 'volume_desc',
      },
    })) as {
      isError?: boolean
      structuredContent: {
        rank_by: string
        combination_count: number
        evaluated_count: number
        succeeded_count: number
        ranking_order: number[]
        variants: Array<{ parameters: Record<string, unknown>; rank: number | null; metrics?: { volume: number } }>
      }
    }

    expect(result.isError).not.toBe(true)
    expect(SWEEP_RANK_OPTIONS).toContain(result.structuredContent.rank_by)
    expect(result.structuredContent.combination_count).toBe(6)
    expect(result.structuredContent.evaluated_count).toBe(6)
    expect(result.structuredContent.succeeded_count).toBe(6)
    const topIndex = result.structuredContent.ranking_order[0]!
    expect(result.structuredContent.variants[topIndex]!.parameters).toEqual({ w: 6, h: 3 })
    expect(result.structuredContent.variants[topIndex]!.metrics?.volume).toBeCloseTo(54, 4)
    expect((await store.getCatalogStats()).buildCount).toBe(0)

    const limited = (await request('tools/call', {
      name: 'openscad_parameter_sweep',
      arguments: {
        source,
        axes: [{ name: 'w', values: [2, 4, 6, 8] }],
        max_combinations: 2,
      },
    })) as { isError?: boolean; structuredContent: { error?: { code: string } } }
    expect(limited.isError).toBe(true)
    expect(limited.structuredContent.error?.code).toBe('invalid_argument')
  })

  it('verifies browser <-> MCP handoff bundles, rejects tampering, and enforces strict CAS', async () => {
    const workspaceBundle = exportWorkspaceHandoffBundle({
      documentId: 'handoff-part',
      fileName: 'handoff-part.scad',
      source: '// @language openscad-viewer/brep-1\ncube(5);',
      revision: 3,
      updatedAt: '2026-11-15T00:00:00.000Z',
      parameterPresets: { default: { size: 5 } },
    })
    expect(workspaceBundle.schema).toBe(OPENSCAD_HANDOFF_BUNDLE_SCHEMA)
    expect(verifyOpenScadHandoffBundle(workspaceBundle).routing).toMatchObject({
      language_contract: 'openscad-viewer/brep-1',
      engine_class: 'brep',
      automatic_fallback: false,
    })

    const tamperedRouting = structuredClone(workspaceBundle)
    ;(tamperedRouting.routing as { engine_class: string }).engine_class = 'mesh'
    expect(() => verifyOpenScadHandoffBundle(tamperedRouting)).toThrow(HandoffBundleValidationError)

    const casWorkspace = prepareWorkspaceHandoffImport(
      workspaceBundle,
      {
        documentId: 'handoff-part',
        source: 'cube(1);',
        revision: 3,
      },
      { expectedWorkspaceRevision: 3 },
    )
    expect(casWorkspace.changed).toBe(true)
    expect(casWorkspace.nextRevision).toBe(4)

    expect(() =>
      prepareWorkspaceHandoffImport(
        workspaceBundle,
        {
          documentId: 'handoff-part',
          source: 'cube(1);',
          revision: 4,
        },
        { expectedWorkspaceRevision: 3 },
      ),
    ).toThrow(HandoffConflictError)

    const { request } = await connectedServer()
    const imported = (await request('tools/call', {
      name: 'openscad_import_handoff_bundle',
      arguments: { bundle: workspaceBundle },
    })) as {
      isError?: boolean
      structuredContent: {
        action: string
        model: { id: string; revision: number }
        source_routing: { language_contract: string; engine_class: string }
      }
    }
    expect(imported.isError).not.toBe(true)
    expect(imported.structuredContent).toMatchObject({
      action: 'created',
      model: { id: 'handoff-part', revision: 0 },
      source_routing: { language_contract: 'openscad-viewer/brep-1', engine_class: 'brep' },
    })

    // Re-importing identical source/name is idempotently unchanged.
    const unchanged = (await request('tools/call', {
      name: 'openscad_import_handoff_bundle',
      arguments: { bundle: workspaceBundle },
    })) as { isError?: boolean; structuredContent: { action: string } }
    expect(unchanged.isError).not.toBe(true)
    expect(unchanged.structuredContent.action).toBe('unchanged')

    // Updated bundle without expected_revision triggers strict revision_conflict.
    const nextBundle = createOpenScadHandoffBundle({
      origin: 'browser-workspace',
      id: 'handoff-part',
      name: 'handoff-part.scad',
      source: '// @language openscad-viewer/brep-1\ncube(8);',
      revision: 4,
    })
    const conflict = (await request('tools/call', {
      name: 'openscad_import_handoff_bundle',
      arguments: { bundle: nextBundle },
    })) as { isError?: boolean; structuredContent: { error?: { code: string } } }
    expect(conflict.isError).toBe(true)
    expect(conflict.structuredContent.error?.code).toBe('revision_conflict')

    // Supplying exact expected_revision updates DuckDB and round-trips via openscad_export_handoff_bundle.
    const updated = (await request('tools/call', {
      name: 'openscad_import_handoff_bundle',
      arguments: { bundle: nextBundle, expected_revision: 0 },
    })) as { isError?: boolean; structuredContent: { action: string; model: { revision: number } } }
    expect(updated.isError).not.toBe(true)
    expect(updated.structuredContent).toMatchObject({
      action: 'updated',
      model: { revision: 1 },
    })

    const exportedBundle = (await request('tools/call', {
      name: 'openscad_export_handoff_bundle',
      arguments: { model_id: 'handoff-part' },
    })) as {
      isError?: boolean
      structuredContent: {
        bundle: { origin: string; document: { source: string; revision: number }; routing: { engine_class: string } }
      }
    }
    expect(exportedBundle.isError).not.toBe(true)
    expect(exportedBundle.structuredContent.bundle).toMatchObject({
      origin: 'mcp-catalog',
      document: { source: '// @language openscad-viewer/brep-1\ncube(8);', revision: 1 },
      routing: { engine_class: 'brep' },
    })
  })

  it('exports real AP242 STEP for brep-1 solids and refuses mesh sources without fallback', async () => {
    const { request } = await connectedServer()
    const exported = (await request('tools/call', {
      name: 'openscad_export',
      arguments: {
        source: '// @language openscad-viewer/brep-1\ncube(6);',
        format: 'step',
        file_name: 'brep_cube',
      },
    })) as {
      isError?: boolean
      structuredContent: { artifact: { format: string; mime_type: string; file_name: string; resource_uri: string } }
    }
    expect(exported.isError).not.toBe(true)
    expect(exported.structuredContent.artifact).toMatchObject({
      format: 'step',
      mime_type: 'model/step',
      file_name: 'brep_cube.step',
    })
    const resource = (await request('resources/read', {
      uri: exported.structuredContent.artifact.resource_uri,
    })) as { contents: Array<{ mimeType: string; text?: string }> }
    expect(resource.contents[0]!.mimeType).toBe('model/step')
    expect(resource.contents[0]!.text).toContain('ISO-10303-21;')
    expect(resource.contents[0]!.text).toContain('MANIFOLD_SOLID_BREP')

    const meshRefusal = (await request('tools/call', {
      name: 'openscad_export',
      arguments: { source: 'cube(6);', format: 'step' },
    })) as {
      isError?: boolean
      structuredContent: {
        error: { code: string; details: { missing_capabilities: string; automatic_fallback: boolean } }
      }
    }
    expect(meshRefusal.isError).toBe(true)
    expect(meshRefusal.structuredContent.error).toMatchObject({
      code: 'capability_unavailable',
      details: { missing_capabilities: 'export.step', automatic_fallback: false },
    })
  })

  it('includes markdown reports in rush_report and rush_interference', async () => {
    const { request } = await connectedServer()
    const report = (await request('tools/call', {
      name: 'rush_report',
      arguments: { document: RUSH_GRAPH_UNITS_EXAMPLE },
    })) as { isError?: boolean; structuredContent: { markdown_summary: string } }
    expect(report.isError).not.toBe(true)
    expect(report.structuredContent.markdown_summary).toContain('# Engineering Summary Report')
    expect(report.structuredContent.markdown_summary).toContain('Total Volume')

    const interference = (await request('tools/call', {
      name: 'rush_interference',
      arguments: { document: RUSH_GRAPH_ASSEMBLY_EXAMPLE },
    })) as { isError?: boolean; structuredContent: { markdown_report: string } }
    expect(interference.isError).not.toBe(true)
    expect(interference.structuredContent.markdown_report).toContain('# Assembly Interference Report')
    expect(interference.structuredContent.markdown_report).toContain('no_volume_overlap')
  })

  it('validates mesh_convert structured success and rejects malformed output payloads', () => {
    const validPayload = {
      format: '3mf',
      file_name: 'cube.3mf',
      mime_type: 'model/3mf',
      byte_length: 128,
      source: {
        format: 'obj',
        byte_length: 64,
        triangle_count: 12,
        vertex_count: 8,
        source_vertex_count: 8,
        degenerate_triangles: 0,
      },
    }
    expect(z.safeParse(meshConvertOutputSchema, validPayload).success).toBe(true)

    const invalidPayload = {
      ...validPayload,
      format: 'step',
      source: { ...validPayload.source, triangle_count: -1 },
    }
    expect(z.safeParse(meshConvertOutputSchema, invalidPayload).success).toBe(false)
  })
})
