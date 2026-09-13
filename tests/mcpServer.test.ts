import { GEOMETRY_MANIFEST_ARCHIVE } from '../src/core/geometryExecution'
import { createHash } from 'node:crypto'
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  fromJsonSchema,
  InMemoryTransport,
  type JSONRPCMessage,
  type McpServer,
} from '@modelcontextprotocol/server'
import { createModelGraphMcpServer as createOpenScadMcpServer } from '../src/mcp/createModelGraphServer'
import { BoundedTransport } from '../src/mcp/boundedTransport'
import {
  persistedFailureStatus,
} from '../src/mcp/createServer'
import { DuckDbModelStore } from '../src/mcp/duckdbModelStore'
import { HeadlessGeometryService, type McpGeometryService } from '../src/mcp/geometryService'
import type { ModelStore } from '../src/mcp/modelStore'
import { canonicalJson } from '../src/mcp/engineManifest'

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

async function connectedServer(options: {
  maxInFlightRequests?: number
  decorateStore?: (store: ModelStore) => ModelStore
  geometry?: McpGeometryService
} = {}) {
  const baseStore = await DuckDbModelStore.open(':memory:')
  const store = options.decorateStore?.(baseStore) ?? baseStore
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()
  const boundedTransport = new BoundedTransport(serverTransport, {
    maxInFlightRequests: options.maxInFlightRequests ?? 8,
    maxPendingOutboundMessages: 24,
  })
  const settledIds = new Set<string | number>()
  const server = createOpenScadMcpServer({
    store,
    geometry: options.geometry,
    onRequestSettled: requestId => {
      settledIds.add(requestId)
      boundedTransport.settleRequest(requestId)
    },
    onRequestCancelled: requestId => boundedTransport.cancelRequest(requestId),
  })
  const pending = new Map<number, (response: JsonRpcResponse) => void>()
  const notifications: string[] = []
  clientTransport.onmessage = message => {
    if ('id' in message && typeof message.id === 'number' && ('result' in message || 'error' in message)) {
      pending.get(message.id)?.(message as JsonRpcResponse)
      pending.delete(message.id)
    } else if ('method' in message && typeof message.method === 'string') {
      notifications.push(message.method)
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
      }, 5_000)
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

  const initialize = await request('initialize', {
    protocolVersion: '2025-11-25',
    capabilities: {},
    clientInfo: { name: 'open-scad-viewer-test', version: '1.0.0' },
  })
  await clientTransport.send({ jsonrpc: '2.0', method: 'notifications/initialized' } as JSONRPCMessage)

  closeables.push(async () => {
    await server.close()
    await baseStore.close()
  })
  return { request, server, store: baseStore, notifications, initialize, clientTransport, settledIds }
}

describe('OpenSCAD MCP server', () => {
  it('does not leak legacy admission when cancellation races the built-in ping handler', async () => {
    const { request, clientTransport, settledIds } = await connectedServer({ maxInFlightRequests: 1 })
    const pingId = 90
    await Promise.all([
      clientTransport.send({ jsonrpc: '2.0', id: pingId, method: 'ping', params: {} }),
      clientTransport.send({
        jsonrpc: '2.0',
        method: 'notifications/cancelled',
        params: { requestId: pingId, reason: 'cancel built-in handler' },
      }),
    ])
    await vi.waitFor(() => expect(settledIds.has(pingId)).toBe(true))
    await new Promise<void>(resolve => setTimeout(resolve, 0))

    const listed = await request('tools/list') as { tools: unknown[] }
    expect(listed.tools.length).toBeGreaterThan(0)
  })

  it('advertises tools and persists a model exposed as an MCP resource', async () => {
    const { request, initialize } = await connectedServer()
    expect(initialize.instructions).toContain('openscad_check')
    expect(initialize.instructions).toContain('First call modelgraph_language')
    const listed = await request('tools/list') as {
      tools: Array<{ name: string; annotations?: { destructiveHint?: boolean; readOnlyHint?: boolean } }>
    }
    expect(listed.tools.map(tool => tool.name).sort()).toEqual([
      'modelgraph_check',
      'modelgraph_compile',
      'modelgraph_export',
      'modelgraph_generate',
      'modelgraph_interference',
      'modelgraph_language',
      'modelgraph_modify',
      'modelgraph_nurbs_build',
      'modelgraph_nurbs_compile',
      'modelgraph_nurbs_evaluate',
      'modelgraph_nurbs_export',
      'modelgraph_nurbs_language',
      'modelgraph_report',
      'modelgraph_set_parameters',
      'modelgraph_svg_export',
      'modelgraph_svg_extrude',
      'modelgraph_svg_preview',
      'modelgraph_text_compile',
      'openscad_analyze',
      'openscad_build_history',
      'openscad_catalog_stats',
      'openscad_check',
      'openscad_compare',
      'openscad_customize',
      'openscad_customize_model',
      'openscad_export',
      'openscad_get_model',
      'openscad_independent_check',
      'openscad_independent_export',
      'openscad_list_engines',
      'openscad_list_model_revisions',
      'openscad_list_models',
      'openscad_official_check',
      'openscad_official_export',
      'openscad_official_status',
      'openscad_save_model',
    ])
    const tools = new Map(listed.tools.map(tool => [tool.name, tool]))
    expect(tools.get('openscad_analyze')?.annotations).toMatchObject({
      readOnlyHint: false,
      destructiveHint: true,
    })
    expect(tools.get('openscad_export')?.annotations).toMatchObject({
      readOnlyHint: false,
      destructiveHint: true,
    })
    expect(tools.get('openscad_check')?.annotations).toMatchObject({
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
    })
    expect(tools.get('openscad_independent_check')?.annotations).toMatchObject({
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: false,
    })
    expect(tools.get('openscad_independent_export')?.annotations).toMatchObject({
      readOnlyHint: false,
      destructiveHint: false,
      idempotentHint: true,
    })
    expect(tools.get('openscad_compare')?.annotations).toMatchObject({
      readOnlyHint: true,
      destructiveHint: false,
    })
    expect(tools.get('openscad_customize_model')?.annotations).toMatchObject({
      readOnlyHint: false,
      destructiveHint: true,
    })

    const called = await request('tools/call', {
      name: 'openscad_save_model',
      arguments: { id: 'integration:part', name: 'Integration', source: 'cube(3);' },
    }) as { structuredContent: { model: { id: string; revision: number; resource_uri: string } } }
    expect(called.structuredContent.model).toMatchObject({
      id: 'integration:part',
      revision: 0,
      resource_uri: 'openscad://models/integration%3Apart',
    })

    const resource = await request('resources/read', {
      uri: called.structuredContent.model.resource_uri,
    }) as {
      contents: Array<{ text: string; mimeType: string }>
    }
    expect(resource.contents).toEqual([expect.objectContaining({
      text: 'cube(3);',
      mimeType: 'text/x-openscad',
    })])

    const capabilities = await request('resources/read', {
      uri: 'openscad://capabilities',
    }) as { contents: Array<{ text: string }> }
    const capabilitiesJson = JSON.parse(capabilities.contents[0].text) as Record<string, unknown>
    expect(capabilitiesJson).toMatchObject({
      protocol_versions: expect.arrayContaining(['2026-07-28', '2025-11-25']),
      geometry_engines: {
        source_directed_routing: true,
        automatic_fallback: false,
        engines: [
          { engine_class: 'mesh', permanent: true, availability: 'available' },
          { engine_class: 'brep', permanent: true, availability: 'unavailable' },
        ],
      },
      geometry_host: {
        id: 'mcp-geometry-host-isolation-v1',
        contract_version: 1,
        scope: 'node-mcp-stdio',
        boundary: 'worker-thread',
        provider_lifecycle: 'disposable-worker-per-job',
        limits: {
          max_admitted_jobs: 8,
          max_concurrent_workers: 1,
          job_deadline_ms: 30_000,
          startup_timeout_ms: 5_000,
          cancellation_grace_ms: 25,
          worker_join_timeout_ms: 1_000,
        },
        enforcement: {
          queue_time_in_deadline: true,
          hard_terminate_after_cancel_grace: true,
          worker_terminal_settles_only_after_join: true,
          next_job_only_after_worker_join: true,
          join_timeout_rejects_and_quarantines: true,
          unjoined_worker_unref_on_quarantine: true,
          host_environment_inherited: false,
          quarantine_after_join_timeout: true,
          worker_reuse: false,
        },
        residual_risks: {
          subprocess_boundary: false,
          os_enforced_memory_limit: false,
          filesystem_sandbox: false,
          network_sandbox: false,
          worker_threads_share_process: true,
        },
      },
      limits: { pending_geometry_jobs: 8 },
      persistence: { arbitrary_sql: false, browser_indexeddb_synchronized: false },
    })
  })

  it('discovers immutable engine manifests and refuses B-rep without mesh fallback', async () => {
    const { request } = await connectedServer()
    const listed = await request('tools/call', {
      name: 'openscad_list_engines',
      arguments: {},
    }) as {
      structuredContent: {
        geometry_engines: {
          automatic_fallback: boolean
          engines: Array<{
            engine_class: string
            availability: string
            manifest_resource_uri: string
          }>
        }
      }
    }
    expect(listed.structuredContent.geometry_engines).toMatchObject({
      automatic_fallback: false,
      engines: [
        {
          engine_class: 'mesh',
          availability: 'available',
          manifest_resource_uri: 'openscad://engines/mesh/capabilities/own-rust-node-v1',
        },
        { engine_class: 'brep', availability: 'unavailable' },
      ],
    })

    const brepManifestUri = listed.structuredContent.geometry_engines.engines[1].manifest_resource_uri
    const manifest = await request('resources/read', { uri: brepManifestUri }) as {
      contents: Array<{ text: string }>
    }
    const manifestJson = JSON.parse(manifest.contents[0].text) as Record<string, unknown>
    expect(manifestJson).toMatchObject({
      engine_class: 'brep',
      permanent: true,
      input_contract: 'semantic-program-required',
      semantic_program_version: 'semantic-program-contract-v1',
      capability_manifest_version: 'brep-contract-v1',
      manifest_digest: expect.stringMatching(/^[a-f0-9]{64}$/),
      capabilities: [],
      planned_capabilities: expect.arrayContaining(['geometry.brep', 'nurbs.surfaces']),
      limits: { sourceCharacters: 250_000 },
      isolation: 'not-deployed',
      deployment: 'not-deployed',
      qualification: {
        status: 'not-qualified',
        record_id: null,
        corpus_version: null,
        target: 'not-deployed',
      },
      dependency: {
        package_name: null,
        sbom_ref: null,
        sbom_sha256: null,
      },
      rollback_compatibility: {
        disable_engine_capability: true,
        source_contract_preserved: true,
        cross_engine_fallback: false,
        minimum_catalog_schema: 3,
      },
      automatic_fallback: false,
    })
    expect(manifestJson).not.toHaveProperty('availability')
    expect(manifestJson).not.toHaveProperty('unavailable_reason')

    const meshManifest = await request('resources/read', {
      uri: 'openscad://engines/mesh/capabilities/own-rust-node-v1',
    }) as { contents: Array<{ text: string }> }
    const meshManifestJson = JSON.parse(meshManifest.contents[0].text) as
      Record<string, unknown>
    expect(meshManifestJson).toMatchObject({
      engine_class: 'mesh',
      capability_manifest_version: 'own-rust-node-v1',
      isolation: 'in-process-serialized',
      dependency: {
        package_name: 'workspace:geometry-bridge',
        license_expression: 'MIT',
        sbom_ref: 'THIRD_PARTY_NOTICES.md',
        sbom_sha256: expect.stringMatching(/^[a-f0-9]{64}$/),
        lockfile_sha256: expect.stringMatching(/^[a-f0-9]{64}$/),
      },
    })
    expect(meshManifestJson).not.toHaveProperty('geometry_host')
    expect(meshManifestJson).not.toHaveProperty('mcp_host_contract_id')
    await expect(request('resources/read', {
      uri: 'openscad://engines/mesh/capabilities/manifold-node-v1',
    })).rejects.toThrow(/not found/i)
    await expect(request('resources/read', {
      uri: 'openscad://engines/brep/capabilities/own-rust-node-v1',
    })).rejects.toThrow(/not found/i)
    await expect(request('resources/read', {
      uri: 'openscad://engines/mesh/capabilities/missing-v1',
    })).rejects.toThrow(/not found/i)

    const parity = await request('resources/read', { uri: 'openscad://parity' }) as {
      contents: Array<{ text: string }>
    }
    const parityJson = JSON.parse(parity.contents[0].text) as {
      policy: { payload: unknown; sha256: string }
      engines: Array<Record<string, unknown>>
      [key: string]: unknown
    }
    expect(parityJson).toMatchObject({
      cross_engine_geometric_equivalence: false,
      mcp_host: {
        id: 'mcp-geometry-host-isolation-v1',
        boundary: 'worker-thread',
        provider_lifecycle: 'disposable-worker-per-job',
        residual_risks: {
          subprocess_boundary: false,
          os_enforced_memory_limit: false,
        },
      },
      policy: { id: 'engine-routing-contract-v1', sha256: expect.stringMatching(/^[a-f0-9]{64}$/) },
      corpus: {
        id: 'browser-mcp-engine-parity',
        version: null,
        sha256: null,
        result_sha256: null,
        qualified_at: null,
      },
      engines: [
        {
          engine_class: 'mesh',
          manifest_digest: expect.stringMatching(/^[a-f0-9]{64}$/),
          kernel_fingerprint: GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'].kernelFingerprint,
          browser_mcp_status: 'qualification-pending',
          mcp_target: 'node-wasm-disposable-worker',
          mcp_isolation: 'in-process-serialized',
          mcp_provider_manifest_isolation: 'in-process-serialized',
          mcp_host_isolation: 'disposable-worker-per-job',
          mcp_host_contract_id: 'mcp-geometry-host-isolation-v1',
        },
        {
          engine_class: 'brep',
          manifest_digest: expect.stringMatching(/^[a-f0-9]{64}$/),
          kernel_fingerprint: 'not-deployed',
          browser_mcp_status: 'not-deployed',
          mcp_provider_manifest_isolation: 'not-deployed',
          mcp_host_isolation: 'not-deployed',
          mcp_host_contract_id: null,
        },
      ],
    })
    expect(createHash('sha256').update(canonicalJson(parityJson.policy.payload)).digest('hex'))
      .toBe(parityJson.policy.sha256)
    expect(parityJson.policy.payload).not.toHaveProperty('mcp_host')
    expect(parityJson.policy.payload).not.toHaveProperty('geometry_host')
    const advertisedEngines = listed.structuredContent.geometry_engines.engines
    expect(parityJson.engines.map(engine => ({
      engine_class: engine.engine_class,
      capability_manifest_version: engine.capability_manifest_version,
      manifest_digest: engine.manifest_digest,
      kernel_fingerprint: engine.kernel_fingerprint,
    }))).toEqual(advertisedEngines.map(engine => ({
      engine_class: engine.engine_class,
      capability_manifest_version: (engine as unknown as Record<string, unknown>).capability_manifest_version,
      manifest_digest: (engine as unknown as Record<string, unknown>).manifest_digest,
      kernel_fingerprint: (engine as unknown as Record<string, unknown>).kernel_fingerprint,
    })))

    const source = '// @language openscad-viewer/brep-1\ncube(1);'
    const checked = await request('tools/call', {
      name: 'openscad_check',
      arguments: { source, quality: 'full' },
    }) as { isError: boolean; structuredContent: { error: { code: string; details: object } } }
    expect(checked).toMatchObject({
      isError: true,
      structuredContent: {
        error: {
          code: 'engine_unavailable',
          details: { engine_class: 'brep', automatic_fallback: false },
        },
      },
    })

    const analyzed = await request('tools/call', {
      name: 'openscad_analyze',
      arguments: { source, quality: 'full' },
    }) as {
      isError: boolean
      structuredContent: {
        error: { code: string }
        build: { status: string; execution: { engine_class: string; evidence: string } }
      }
    }
    expect(analyzed).toMatchObject({
      isError: true,
      structuredContent: {
        error: { code: 'engine_unavailable' },
        build: {
          status: 'failed',
          execution: { engine_class: 'brep', evidence: 'planned' },
        },
      },
    })
  })

  it('returns a schema-valid public error when engine discovery fails', async () => {
    const log = vi.spyOn(console, 'error').mockImplementation(() => undefined)
    const geometry = new HeadlessGeometryService()
    vi.spyOn(geometry, 'capabilities').mockRejectedValue(
      new Error('synthetic engine discovery failure'),
    )
    const { request } = await connectedServer({ geometry })
    const listed = await request('tools/list') as {
      tools: Array<{ name: string; outputSchema?: Record<string, unknown> }>
    }
    const outputSchema = listed.tools.find(tool => (
      tool.name === 'openscad_list_engines'
    ))?.outputSchema

    const failed = await request('tools/call', {
      name: 'openscad_list_engines',
      arguments: {},
    }) as {
      isError: boolean
      structuredContent: {
        error: { code: string; retryable: boolean; correlation_id: string }
      }
    }

    expect(failed).toMatchObject({
      isError: true,
      structuredContent: {
        error: {
          code: 'internal_error',
          retryable: true,
          correlation_id: expect.any(String),
        },
      },
    })
    expect(outputSchema).toBeDefined()
    const validation = await fromJsonSchema(outputSchema!)['~standard']
      .validate(failed.structuredContent)
    expect(validation).not.toHaveProperty('issues')
    log.mockRestore()
  })

  it('keeps runtime provenance when persistence fails after geometry completed', async () => {
    const log = vi.spyOn(console, 'error').mockImplementation(() => undefined)
    let recordAttempts = 0
    const { request } = await connectedServer({
      decorateStore: store => new Proxy(store, {
        get(target, property) {
          if (property === 'recordBuild') {
            return async (...args: Parameters<ModelStore['recordBuild']>) => {
              recordAttempts++
              if (recordAttempts === 1) throw new Error('injected persistence failure')
              return await target.recordBuild(...args)
            }
          }
          const value = Reflect.get(target, property, target) as unknown
          return typeof value === 'function' ? value.bind(target) : value
        },
      }),
    })

    const result = await request('tools/call', {
      name: 'openscad_analyze',
      arguments: { source: 'cube(1);', quality: 'full' },
    }) as {
      isError: boolean
      structuredContent: {
        execution: { engine_class: string; evidence: string }
        build: { status: string; execution: { engine_class: string; evidence: string } }
        error: { code: string }
      }
    }
    expect(result).toMatchObject({
      isError: true,
      structuredContent: {
        execution: { engine_class: 'mesh', evidence: 'runtime' },
        build: {
          status: 'failed',
          execution: { engine_class: 'mesh', evidence: 'runtime' },
        },
        error: { code: 'internal_error' },
      },
    })
    expect(recordAttempts).toBe(2)
    log.mockRestore()
  })

  it('does not rewrite an unrelated late failure as cancellation after the request signal aborts', () => {
    const request = new AbortController()
    request.abort('client stopped waiting')
    expect(request.signal.aborted).toBe(true)
    expect(persistedFailureStatus(new Error('unrelated provider failure settled after abort')))
      .toBe('failed')
    expect(persistedFailureStatus(new DOMException('cancelled', 'AbortError')))
      .toBe('cancelled')
  })

  it('advertises the source selector as an inspectable JSON Schema XOR', async () => {
    const { request } = await connectedServer()
    const listed = await request('tools/list') as {
      tools: Array<{
        name: string
        inputSchema: {
          anyOf?: Array<{
            required?: string[]
            properties?: Record<string, unknown>
          }>
        }
      }>
    }
    const analyze = listed.tools.find(tool => tool.name === 'openscad_analyze')
    const variants = analyze?.inputSchema.anyOf

    expect(variants).toHaveLength(2)
    expect(variants?.map(variant => variant.required)).toEqual([['source'], ['model_id']])
    expect(variants?.[0].properties?.model_id).toEqual({ not: {} })
    expect(variants?.[1].properties?.source).toEqual({ not: {} })

    const neither = await request('tools/call', {
      name: 'openscad_analyze',
      arguments: { quality: 'full' },
    }) as { isError: boolean; content: Array<{ text: string }> }
    const both = await request('tools/call', {
      name: 'openscad_analyze',
      arguments: { source: 'cube(1);', model_id: 'integration:part' },
    }) as { isError: boolean; content: Array<{ text: string }> }
    const inlineRevision = await request('tools/call', {
      name: 'openscad_check',
      arguments: { source: 'cube(1);', revision: 0 },
    }) as { isError: boolean; content: Array<{ text: string }> }

    expect(neither).toMatchObject({ isError: true })
    expect(neither.content[0].text).toMatch(/Input validation error/)
    expect(both).toMatchObject({ isError: true })
    expect(both.content[0].text).toMatch(/Input validation error/)
    expect(inlineRevision).toMatchObject({ isError: true })
    expect(inlineRevision.content[0].text).toMatch(/Input validation error/)
  })

  it('rejects oversized Customizer strings before response amplification', async () => {
    const { request } = await connectedServer()
    const called = await request('tools/call', {
      name: 'openscad_customize',
      arguments: {
        source: 'label = "";',
        values: { label: 'x'.repeat(64 * 1024 + 1) },
      },
    }) as { isError: boolean; content: Array<{ text: string }> }

    expect(called).toMatchObject({ isError: true })
    expect(called.content[0].text).toMatch(/Input validation error/)
  })

  it('fails closed on every caller-supplied geometry backend override field', async () => {
    const { request } = await connectedServer()
    for (const [field, value] of [
      ['engine', 'brep'],
      ['engine_class', 'brep'],
      ['backend', 'rust'],
      ['kernel', 'rust-brep-reserved-v1'],
      ['provider', 'brep'],
      ['automatic_fallback', true],
    ] as const) {
      const called = await request('tools/call', {
        name: 'openscad_check',
        arguments: { source: 'cube(1);', quality: 'preview', [field]: value },
      }) as { isError: boolean; content: Array<{ text: string }> }
      expect(called, field).toMatchObject({ isError: true })
      expect(called.content[0].text, field).toMatch(/Input validation error/)

      const customized = await request('tools/call', {
        name: 'openscad_customize_model',
        arguments: {
          model_id: 'override:must-not-resolve',
          expected_revision: 0,
          values: {},
          [field]: value,
        },
      }) as { isError: boolean; content: Array<{ text: string }> }
      expect(customized, `customize_model:${field}`).toMatchObject({ isError: true })
      expect(customized.content[0].text, `customize_model:${field}`).toMatch(/Input validation error/)
    }
  })

  it('returns a machine-readable revision conflict with recovery details', async () => {
    const { request } = await connectedServer()
    const listed = await request('tools/list') as {
      tools: Array<{ name: string; outputSchema?: Record<string, unknown> }>
    }
    const outputSchema = listed.tools.find(tool => tool.name === 'openscad_save_model')?.outputSchema
    expect(outputSchema).toBeDefined()
    await request('tools/call', {
      name: 'openscad_save_model',
      arguments: { id: 'conflict:part', name: 'Conflict', source: 'cube(1);' },
    })

    const conflict = await request('tools/call', {
      name: 'openscad_save_model',
      arguments: {
        id: 'conflict:part',
        name: 'Conflict',
        source: 'cube(2);',
        expected_revision: 4,
      },
    }) as {
      isError: boolean
      structuredContent: {
        error: {
          code: string
          retryable: boolean
          details: Record<string, unknown>
        }
      }
    }

    expect(conflict).toMatchObject({
      isError: true,
      structuredContent: {
        error: {
          code: 'revision_conflict',
          retryable: true,
          details: {
            model_id: 'conflict:part',
            expected_revision: 4,
            actual_revision: 0,
          },
        },
      },
    })
    const validation = await fromJsonSchema(outputSchema!)['~standard'].validate(conflict.structuredContent)
    expect(validation).not.toHaveProperty('issues')
  })

  it('customizes and saves a model through one revision-guarded tool call', async () => {
    const { request } = await connectedServer()
    await request('tools/call', {
      name: 'openscad_save_model',
      arguments: {
        id: 'customize:part',
        name: 'Customizer part',
        source: 'size = 1; // [1:1:5]\ncube(size);',
      },
    })

    const customized = await request('tools/call', {
      name: 'openscad_customize_model',
      arguments: {
        model_id: 'customize:part',
        expected_revision: 0,
        values: { size: 3 },
      },
    }) as {
      structuredContent: {
        model: { revision: number }
        applied: string[]
        parameters: Array<{ name: string; value: number }>
      }
    }
    expect(customized.structuredContent).toMatchObject({
      model: { revision: 1 },
      applied: ['size'],
      parameters: [expect.objectContaining({ name: 'size', value: 3 })],
    })

    const stored = await request('tools/call', {
      name: 'openscad_get_model',
      arguments: { id: 'customize:part' },
    }) as { structuredContent: { model: { source: string; revision: number } } }
    expect(stored.structuredContent.model).toMatchObject({ revision: 1 })
    expect(stored.structuredContent.model.source).toContain('size = 3;')

    const stale = await request('tools/call', {
      name: 'openscad_customize_model',
      arguments: {
        model_id: 'customize:part',
        expected_revision: 0,
        values: { size: 4 },
      },
    }) as { isError: boolean; structuredContent: { error: { code: string } } }
    expect(stale).toMatchObject({
      isError: true,
      structuredContent: { error: { code: 'revision_conflict' } },
    })
  })

  it('lists immutable model revisions and serves encoded revision resources', async () => {
    const { request } = await connectedServer()
    await request('tools/call', {
      name: 'openscad_save_model',
      arguments: { id: 'revision:part', name: 'Revision part', source: 'cube(1);' },
    })
    await request('tools/call', {
      name: 'openscad_save_model',
      arguments: {
        id: 'revision:part',
        name: 'Revision part',
        source: 'cube(2);',
        expected_revision: 0,
      },
    })

    const listed = await request('tools/call', {
      name: 'openscad_list_model_revisions',
      arguments: { id: 'revision:part', limit: 10 },
    }) as {
      structuredContent: {
        revisions: Array<{ revision: number; resource_uri: string }>
      }
    }
    expect(listed.structuredContent.revisions).toEqual([
      expect.objectContaining({ revision: 1 }),
      expect.objectContaining({
        revision: 0,
        resource_uri: 'openscad://models/revision%3Apart/revisions/0',
      }),
    ])

    const original = await request('resources/read', {
      uri: 'openscad://models/revision%3Apart/revisions/0',
    }) as { contents: Array<{ text: string; mimeType: string }> }
    expect(original.contents).toEqual([expect.objectContaining({
      text: 'cube(1);',
      mimeType: 'text/x-openscad',
    })])

    const historical = await request('tools/call', {
      name: 'openscad_get_model',
      arguments: { id: 'revision:part', revision: 0 },
    }) as { structuredContent: { model: { source: string; revision: number; resource_uri: string } } }
    expect(historical.structuredContent.model).toMatchObject({
      source: 'cube(1);',
      revision: 0,
      resource_uri: 'openscad://models/revision%3Apart/revisions/0',
    })
  })

  it('checks and rebuilds an exact historical revision without silently selecting HEAD', async () => {
    const { request } = await connectedServer()
    await request('tools/call', {
      name: 'openscad_save_model',
      arguments: { id: 'history:part', name: 'History', source: 'cube(1);' },
    })
    await request('tools/call', {
      name: 'openscad_save_model',
      arguments: {
        id: 'history:part',
        name: 'History',
        source: 'cube(2);',
        expected_revision: 0,
      },
    })

    const historical = await request('tools/call', {
      name: 'openscad_check',
      arguments: { model_id: 'history:part', revision: 0, quality: 'full' },
    }) as {
      structuredContent: {
        model_revision: number
        analysis: { volume: number }
      }
    }
    expect(historical.structuredContent).toMatchObject({
      model_revision: 0,
      analysis: { volume: 1 },
    })

    const compared = await request('tools/call', {
      name: 'openscad_compare',
      arguments: {
        left: { model_id: 'history:part', revision: 0 },
        right: { model_id: 'history:part', revision: 1 },
        quality: 'full',
      },
    }) as {
      structuredContent: {
        left: { volume: number }
        right: { volume: number }
        delta: { volume: number; surface_area: number; dimensions: number[] }
      }
    }
    expect(compared.structuredContent).toMatchObject({
      left: { volume: 1 },
      right: { volume: 8 },
      delta: { volume: 7, surface_area: 18, dimensions: [1, 1, 1] },
    })

    const failedComparison = await request('tools/call', {
      name: 'openscad_compare',
      arguments: {
        left: { source: 'cube(1);' },
        right: { source: '// @language unsupported/future\ncube(2);' },
        quality: 'full',
      },
    }) as {
      isError: boolean
      structuredContent: {
        error: { code: string }
        comparison_failure: {
          failed_side: string
          completed_left: {
            execution: { engine_class: string; evidence: string }
            volume: number
          }
        }
      }
    }
    expect(failedComparison).toMatchObject({
      isError: true,
      structuredContent: {
        error: { code: 'language_contract_unsupported' },
        comparison_failure: {
          failed_side: 'right',
          completed_left: {
            execution: { engine_class: 'mesh', evidence: 'runtime' },
            volume: 1,
          },
        },
      },
    })

    const emptyHistory = await request('tools/call', {
      name: 'openscad_build_history',
      arguments: { model_id: 'history:part' },
    }) as { structuredContent: { builds: unknown[] } }
    expect(emptyHistory.structuredContent.builds).toEqual([])

    const current = await request('tools/call', {
      name: 'openscad_check',
      arguments: { model_id: 'history:part', quality: 'full' },
    }) as { structuredContent: { model_revision: number; analysis: { volume: number } } }
    expect(current.structuredContent).toMatchObject({
      model_revision: 1,
      analysis: { volume: 8 },
    })

    const recorded = await request('tools/call', {
      name: 'openscad_analyze',
      arguments: { model_id: 'history:part', revision: 0, quality: 'full' },
    }) as {
      structuredContent: {
        build: { model_revision: number }
        analysis: { volume: number }
      }
    }
    expect(recorded.structuredContent).toMatchObject({
      build: { model_revision: 0 },
      analysis: { volume: 1 },
    })
  })

  it('advertises guided review and customization prompts', async () => {
    const { request } = await connectedServer()
    const listed = await request('prompts/list') as {
      prompts: Array<{ name: string; description?: string }>
    }
    expect(listed.prompts.map(prompt => prompt.name).sort()).toEqual([
      'openscad_customize_workflow',
      'openscad_review_model',
    ])

    const prompt = await request('prompts/get', {
      name: 'openscad_review_model',
      arguments: { model_id: 'review:part', revision: '3', goal: 'Check dimensions' },
    }) as { messages: Array<{ content: { text: string } }> }
    expect(prompt.messages[0].content.text).toContain('{"model_id":"review:part","revision":3}')
    expect(prompt.messages[0].content.text).toContain('Do not save or export')
  })

  it('compiles through MCP and records a DuckDB build result', async () => {
    const { request } = await connectedServer()
    const called = await request('tools/call', {
      name: 'openscad_analyze',
      arguments: { source: 'cube(2);', quality: 'full' },
    }) as {
      structuredContent: {
        build: { status: string; resource_uri: string }
        analysis: { triangle_count: number; volume: number }
      }
      content: Array<{ type: string; text: string }>
    }
    expect(called.structuredContent.analysis).toMatchObject({ triangle_count: 12, volume: 8 })
    expect(called.structuredContent.build).toMatchObject({
      status: 'succeeded',
      execution: {
        language_contract: 'legacy/current',
        engine_class: 'mesh',
        evidence: 'runtime',
        automatic_fallback: false,
      },
    })
    expect(JSON.parse(called.content[0].text)).toMatchObject({
      build: { status: 'succeeded' },
      analysis: { triangle_count: 12, volume: 8 },
    })

    const history = await request('tools/call', {
      name: 'openscad_build_history',
      arguments: { limit: 10 },
    }) as { structuredContent: { builds: Array<{ status: string }> } }
    expect(history.structuredContent.builds).toHaveLength(1)
    expect(history.structuredContent.builds[0].status).toBe('succeeded')
  })

  it('records own-rust runtime provenance on new executions', async () => {
    const { request } = await connectedServer()
    const source = 'cube(1);'
    const current = await request('tools/call', {
      name: 'openscad_check',
      arguments: { source, quality: 'full' },
    }) as { structuredContent: { analysis: { execution: Record<string, unknown> } } }
    expect(current.structuredContent.analysis.execution).toMatchObject({
      capability_manifest_version: 'own-rust-node-v1',
      manifest_digest: GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'].manifestDigest,
      evidence: 'runtime',
    })

    const listedTools = await request('tools/list') as {
      tools: Array<{ name: string; outputSchema?: Record<string, unknown> }>
    }
    const historySchema = listedTools.tools.find(tool => (
      tool.name === 'openscad_build_history'
    ))?.outputSchema
    expect(historySchema).toBeDefined()
    const history = await request('tools/call', {
      name: 'openscad_build_history',
      arguments: { limit: 10 },
    }) as { structuredContent: { builds: Array<{ execution: Record<string, unknown> }> } }
    const validation = await fromJsonSchema(historySchema!)['~standard']
      .validate(history.structuredContent)
    expect(validation).not.toHaveProperty('issues')
  })

  it('reports DuckDB catalog usage and quota limits without exposing SQL', async () => {
    const { request } = await connectedServer()
    await request('tools/call', {
      name: 'openscad_save_model',
      arguments: { id: 'stats:part', name: 'Stats', source: 'cube(2);' },
    })
    await request('tools/call', {
      name: 'openscad_analyze',
      arguments: { model_id: 'stats:part', quality: 'full' },
    })

    const result = await request('tools/call', {
      name: 'openscad_catalog_stats',
      arguments: {},
    }) as {
      structuredContent: {
        stats: {
          models: number
          revisions: number
          builds: number
          builds_by_status: { succeeded: number }
          limits: { models: number; artifact_bytes: number }
        }
      }
    }
    expect(result.structuredContent.stats).toMatchObject({
      models: 1,
      revisions: 1,
      builds: 1,
      builds_by_status: { succeeded: 1 },
      limits: { models: 500, artifact_bytes: 64 * 1024 * 1024 },
    })
  })

  it('exports a bounded DuckDB artifact and serves it as an MCP resource', async () => {
    const { request } = await connectedServer()
    const called = await request('tools/call', {
      name: 'openscad_export',
      arguments: { source: 'cube(1);', format: 'stl', file_name: 'cube' },
    }) as {
      structuredContent: {
        artifact: { resource_uri: string; file_name: string; byte_length: number }
      }
      content: Array<{ type: string; uri?: string }>
    }
    expect(called.structuredContent.artifact).toMatchObject({
      file_name: 'cube.stl',
      byte_length: 84 + 12 * 50,
    })
    expect(called.content).toContainEqual(expect.objectContaining({
      type: 'resource_link',
      uri: called.structuredContent.artifact.resource_uri,
    }))

    const resource = await request('resources/read', {
      uri: called.structuredContent.artifact.resource_uri,
    }) as { contents: Array<{ blob: string; mimeType: string }> }
    expect(resource.contents[0].mimeType).toBe('model/stl')
    expect(Buffer.from(resource.contents[0].blob, 'base64')).toHaveLength(84 + 12 * 50)
  })

  it('truncates long Unicode export names without splitting a surrogate pair', async () => {
    const { request } = await connectedServer()
    const called = await request('tools/call', {
      name: 'openscad_export',
      arguments: {
        source: 'cube(1);',
        format: 'stl',
        file_name: `${'a'.repeat(250)}😀`,
      },
    }) as { structuredContent: { artifact: { file_name: string } } }

    expect(called.structuredContent.artifact.file_name).toBe(`${'a'.repeat(250)}.stl`)
  })

  it('returns positioned compiler failures and records them in build history', async () => {
    const { request } = await connectedServer()
    const called = await request('tools/call', {
      name: 'openscad_analyze',
      arguments: { source: 'cube(1);\nunsupported(); // SECRET_TOKEN' },
    }) as {
      isError: boolean
      content: Array<{ type: string; text: string }>
      structuredContent: { error: { code: string; line: number; column: number } }
    }
    expect(called.isError).toBe(true)
    expect(called.structuredContent.error).toMatchObject({
      code: 'source_syntax_error',
      line: 2,
      column: 1,
    })
    expect(JSON.parse(called.content[0].text)).toMatchObject({
      build: {
        status: 'failed',
        execution: { engine_class: 'mesh', evidence: 'runtime' },
      },
      error: { code: 'source_syntax_error', line: 2, column: 1 },
    })
    expect(called.content[0].text).not.toContain('SECRET_TOKEN')

    const history = await request('tools/call', {
      name: 'openscad_build_history',
      arguments: {},
    }) as { structuredContent: { builds: Array<{ status: string; error: { line: number } }> } }
    expect(history.structuredContent.builds).toEqual([
      expect.objectContaining({ status: 'failed', error: expect.objectContaining({ line: 2 }) }),
    ])
    expect(JSON.stringify(history.structuredContent)).not.toContain('SECRET_TOKEN')
  })

  it('emits resource list_changed notifications after persisted MCP writes', async () => {
    const { request, notifications } = await connectedServer()
    const listChangedCount = () => notifications.filter(method => (
      method === 'notifications/resources/list_changed'
    )).length

    await request('tools/call', {
      name: 'openscad_save_model',
      arguments: { id: 'notify:part', name: 'Notify', source: 'cube(1);' },
    })
    await expect.poll(listChangedCount).toBe(1)

    await request('tools/call', {
      name: 'openscad_analyze',
      arguments: { model_id: 'notify:part', quality: 'full' },
    })
    await expect.poll(listChangedCount).toBe(2)

    await request('tools/call', {
      name: 'openscad_export',
      arguments: { model_id: 'notify:part', format: 'stl', file_name: 'notify.stl' },
    })
    await expect.poll(listChangedCount).toBe(3)
  })
})

describe('ModelGraph MCP language workflow', () => {
  it('serves a model-ready schema and compiles, edits and checks a structured model', async () => {
    const { MODELGRAPH_EXAMPLE } = await import('../src/services/modelGraph')
    const { request, initialize } = await connectedServer()
    expect(initialize.instructions).toContain('openscad_check')
    expect(initialize.instructions).toContain('First call modelgraph_language')
    const resource = await request('resources/read', { uri: 'openscad://language/modelgraph-1' }) as { contents: Array<{ text: string }> }
    const contract = JSON.parse(resource.contents[0].text)
    expect(contract.language).toBe('modelgraph/1')
    expect(contract.schema.additionalProperties).toBe(false)
    expect(contract.guide).toContain('lexical closures')
    expect(contract.guide).toContain('type_policy:')
    const modified = await request('tools/call', {name:'modelgraph_modify', arguments:{
      document:{language:'modelgraph/1',units:'mm',parameters:[],nodes:[{id:'b',op:'box',size:[10,10,10]}],root:'b'},
      modification:{operation:'split',axis:'z',position:4},
    }}) as {isError?:boolean;structuredContent:{results:Array<{analysis:{volume:number}}>}}
    expect(modified.isError).not.toBe(true)
    expect(modified.structuredContent.results[0].analysis.volume).toBeCloseTo(400,6)
    expect(modified.structuredContent.results[1].analysis.volume).toBeCloseTo(600,6)

    const assembly = await request('tools/call', { name: 'modelgraph_check', arguments: { document: contract.assembly_example } }) as { isError?: boolean; structuredContent: { assembly_components: Array<{ id: string; matrix: number[] }>; analysis: { meshCount: number } } }
    expect(assembly.isError).toBe(false)
    expect(assembly.structuredContent.analysis.meshCount).toBe(2)
    expect(assembly.structuredContent.assembly_components[1].matrix[11]).toBeCloseTo(5.3)
    const sketch = await request('tools/call', { name: 'modelgraph_check', arguments: { document: contract.sketch_example } }) as { isError?: boolean; structuredContent: { sketch_solutions: Array<{ status: string }>; analysis: { volume: number } } }
    expect(sketch.isError).toBe(false)
    expect(sketch.structuredContent.sketch_solutions[0].status).toBe('solved')
    expect(sketch.structuredContent.analysis.volume).toBeCloseTo(600, 3)
    const typed = await request('tools/call', { name: 'modelgraph_compile', arguments: { document: contract.units_example } }) as { structuredContent: { document: unknown; document_sha256: string; constraint_report: Array<{ passed: boolean }> } }
    expect(typed.structuredContent.constraint_report[0].passed).toBe(true)
    const rejected = await request('tools/call', { name: 'modelgraph_set_parameters', arguments: { document: typed.structuredContent.document, expected_document_sha256: typed.structuredContent.document_sha256, updates: [{ id: 'wall', value: 0.6 }] } }) as { isError: boolean; structuredContent: { error: { code: string; details: unknown[] } } }
    expect(rejected.isError).toBe(true)
    expect(rejected.structuredContent.error.code).toBe('constraint_failed')
    expect(rejected.structuredContent.error.details).toContainEqual(expect.objectContaining({ id: 'minimumWall', actual: 0.6, expected: 1.2, passed: false }))
    const functional = await request('tools/call', { name: 'modelgraph_check', arguments: { document: contract.functional_example } }) as { isError?: boolean; structuredContent: { analysis: { volume: number } } }
    expect(functional.isError).toBe(false)
    expect(functional.structuredContent.analysis.volume).toBeCloseTo(24)
    const report = await request('tools/call', { name: 'modelgraph_report', arguments: { document: contract.units_example } }) as { content: Array<{ type: string }>; structuredContent: { images_status: string; references: Array<{ node_id: string | null }> } }
    expect(report.structuredContent.images_status).toBe('rendered')
    expect(report.content.filter(item => item.type === 'image')).toHaveLength(3)
    expect(report.structuredContent.references.some(item => item.node_id === 'plate')).toBe(true)
    const compiled = await request('tools/call', { name: 'modelgraph_compile', arguments: { document: MODELGRAPH_EXAMPLE } }) as { structuredContent: { document: unknown; document_sha256: string } }
    const changed = await request('tools/call', { name: 'modelgraph_set_parameters', arguments: { document: compiled.structuredContent.document, expected_document_sha256: compiled.structuredContent.document_sha256, updates: [{ id: 'width', value: 50 }] } }) as { structuredContent: { document: unknown } }
    const checked = await request('tools/call', { name: 'modelgraph_check', arguments: { document: changed.structuredContent.document } }) as { isError?: boolean; structuredContent: { analysis: { volume: number } } }
    expect(checked.isError).toBe(false)
    expect(checked.structuredContent.analysis.volume).toBeGreaterThan(11000)
  })
})

describe('Mechanical generators over MCP',()=>{
  it('generates readable models, returns individual planetary parts and exports a thread',async()=>{
    const {request}=await connectedServer()
    const resource=await request('resources/read',{uri:'openscad://language/modelgraph-mechanical'}) as {contents:Array<{text:string}>}
    expect(JSON.parse(resource.contents[0].text).examples.thread).toBeTruthy()
    for(const kind of ['gear','planetary_gears','thread']) {
      const response=await request('tools/call',{name:'modelgraph_generate',arguments:{kind}}) as {isError?:boolean;content:Array<{type:string}>;structuredContent:{document:unknown;analysis:{volume:number;meshCount:number};mechanical_reports:unknown[];mechanical_parts:Array<{document:unknown}>}}
      expect(response.isError,JSON.stringify(response.structuredContent).slice(0,500)).not.toBe(true)
      expect(response.structuredContent.analysis.volume).toBeGreaterThan(0)
      expect(response.content.filter(c=>c.type==='image')).toHaveLength(3)
      expect(response.structuredContent.mechanical_reports).toHaveLength(1)
      if(kind==='planetary_gears')expect(response.structuredContent.mechanical_parts).toHaveLength(5)
      const exported=await request('tools/call',{name:'modelgraph_export',arguments:{document:response.structuredContent.document,format:'3mf'}}) as {isError?:boolean;content:Array<{resource:{blob:string}}>}
      expect(exported.isError).not.toBe(true)
      expect(Buffer.from(exported.content[0].resource.blob,'base64').readUInt32LE(0)).toBe(0x04034b50)
    }
    const rejected=await request('tools/call',{name:'modelgraph_generate',arguments:{kind:'gear',teeth:8}}) as {isError:boolean;structuredContent:{error:{code:string}}}
    expect(rejected.isError).toBe(true)
    expect(rejected.structuredContent.error.code).toBe('invalid_mechanical_geometry')
  },30000)
})
