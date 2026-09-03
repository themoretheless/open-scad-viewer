import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  fromJsonSchema,
  InMemoryTransport,
  type JSONRPCMessage,
} from '@modelcontextprotocol/server'
import { serveStdio, type StdioServerHandle } from '@modelcontextprotocol/server/stdio'
import { BoundedTransport, MAX_MCP_SUBSCRIPTIONS } from '../src/mcp/boundedTransport'
import { createOpenScadMcpServer } from '../src/mcp/createServer'
import { DuckDbModelStore } from '../src/mcp/duckdbModelStore'
import { HeadlessGeometryService } from '../src/mcp/geometryService'

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

const modernMeta = {
  'io.modelcontextprotocol/protocolVersion': '2026-07-28',
  'io.modelcontextprotocol/clientInfo': { name: 'open-scad-viewer-modern-test', version: '1.0.0' },
  'io.modelcontextprotocol/clientCapabilities': {},
}

async function connectedModernServer(options: {
  geometry?: HeadlessGeometryService
  maxInFlightRequests?: number
  maxSubscriptions?: number
} = {}) {
  const store = await DuckDbModelStore.open(':memory:')
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()
  const boundedTransport = new BoundedTransport(serverTransport, {
    maxInFlightRequests: options.maxInFlightRequests ?? 8,
    maxPendingOutboundMessages: 24,
  })
  const pending = new Map<number, (response: JsonRpcResponse) => void>()
  const messages: JSONRPCMessage[] = []
  const settledIds = new Set<string | number>()
  let nextId = 1

  clientTransport.onmessage = message => {
    messages.push(message)
    if ('id' in message && typeof message.id === 'number' && ('result' in message || 'error' in message)) {
      pending.get(message.id)?.(message as JsonRpcResponse)
      pending.delete(message.id)
    }
  }
  await clientTransport.start()
  const handle: StdioServerHandle = serveStdio(
    () => createOpenScadMcpServer({
      store,
      geometry: options.geometry,
      onRequestSettled: requestId => {
        settledIds.add(requestId)
        boundedTransport.settleRequest(requestId)
      },
      onRequestCancelled: requestId => boundedTransport.cancelRequest(requestId),
    }),
    {
      transport: boundedTransport,
      maxSubscriptions: options.maxSubscriptions ?? MAX_MCP_SUBSCRIPTIONS,
    },
  )

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
    // In-memory delivery resolves the client waiter during inner.send(); let
    // the bounded transport's send-finally retire the request before issuing
    // the next one, matching a real stdio write/drain boundary.
    await new Promise<void>(resolve => queueMicrotask(resolve))
    if (value.error) throw new Error(`${value.error.code}: ${value.error.message}`)
    return value.result ?? {}
  }

  const modernRequest = (method: string, params: Record<string, unknown> = {}) => request(method, {
    ...params,
    _meta: modernMeta,
  })

  const sendModernRequest = (id: number, method: string, params: Record<string, unknown> = {}) => (
    clientTransport.send({
      jsonrpc: '2.0',
      id,
      method,
      params: { ...params, _meta: modernMeta },
    } as JSONRPCMessage)
  )

  const sendModernNotification = (method: string, params: Record<string, unknown>) => (
    clientTransport.send({
      jsonrpc: '2.0',
      method,
      params: { ...params, _meta: modernMeta },
    } as JSONRPCMessage)
  )

  closeables.push(async () => {
    await handle.close()
    await store.close()
  })
  return {
    modernRequest,
    request,
    sendModernRequest,
    sendModernNotification,
    messages,
    settledIds,
  }
}

class BlockingGeometryService extends HeadlessGeometryService {
  readonly started: Promise<void>
  readonly released: Promise<void>
  private markStarted!: () => void
  private releaseGate!: () => void

  constructor() {
    super()
    this.started = new Promise(resolve => { this.markStarted = resolve })
    this.released = new Promise(resolve => { this.releaseGate = resolve })
  }

  release() {
    this.releaseGate()
  }

  override async analyze(...args: Parameters<HeadlessGeometryService['analyze']>) {
    this.markStarted()
    await this.released
    return await super.analyze(...args)
  }
}

describe('OpenSCAD MCP server, protocol 2026-07-28', () => {
  it('discovers with the required modern envelope and returns cacheable tool/resource listings', async () => {
    const { modernRequest, request } = await connectedModernServer()

    const discovered = await modernRequest('server/discover')
    expect(discovered.supportedVersions).toContain('2026-07-28')
    expect(discovered.capabilities).toMatchObject({
      tools: { listChanged: true },
      resources: { listChanged: true },
    })
    expect(discovered).toMatchObject({ ttlMs: 300_000, cacheScope: 'private' })
    expect(discovered._meta).toMatchObject({
      'io.modelcontextprotocol/serverInfo': { name: 'open-scad-viewer', version: '0.1.0' },
    })

    const tools = await modernRequest('tools/list')
    expect(tools).toMatchObject({ ttlMs: 300_000, cacheScope: 'private' })
    expect(tools._meta).toMatchObject({
      'io.modelcontextprotocol/serverInfo': { name: 'open-scad-viewer', version: '0.1.0' },
    })
    expect((tools.tools as Array<{ name: string }>).map(tool => tool.name)).toEqual(expect.arrayContaining([
      'openscad_analyze',
      'openscad_check',
    ]))
    const getModelTool = (tools.tools as Array<{
      name: string
      outputSchema?: Record<string, unknown>
    }>).find(tool => tool.name === 'openscad_get_model')
    const listEnginesTool = (tools.tools as Array<{
      name: string
      outputSchema?: Record<string, unknown>
    }>).find(tool => tool.name === 'openscad_list_engines')
    const analyzeTool = (tools.tools as Array<{
      name: string
      outputSchema?: Record<string, unknown>
    }>).find(tool => tool.name === 'openscad_analyze')
    const missing = await modernRequest('tools/call', {
      name: 'openscad_get_model',
      arguments: { id: 'missing:modern' },
    })
    expect(missing).toMatchObject({
      isError: true,
      structuredContent: { error: { code: 'model_not_found' } },
    })
    const validation = await fromJsonSchema(getModelTool!.outputSchema!)['~standard']
      .validate(missing.structuredContent)
    expect(validation).not.toHaveProperty('issues')

    const prompts = await modernRequest('prompts/list')
    expect(prompts).toMatchObject({ ttlMs: 300_000, cacheScope: 'private' })
    expect((prompts.prompts as Array<{ name: string }>).map(prompt => prompt.name)).toEqual(expect.arrayContaining([
      'openscad_customize_workflow',
      'openscad_review_model',
    ]))

    const resources = await modernRequest('resources/list')
    expect(resources).toMatchObject({ ttlMs: 0, cacheScope: 'private' })
    expect(resources._meta).toMatchObject({
      'io.modelcontextprotocol/serverInfo': { name: 'open-scad-viewer', version: '0.1.0' },
    })
    expect((resources.resources as Array<{ uri: string }>).map(resource => resource.uri))
      .toEqual(expect.arrayContaining([
        'openscad://examples/basic',
        'openscad://engines/manifold/capabilities/manifold-node-v1',
        'openscad://engines/manifold/capabilities/manifold-node-v2',
        'openscad://engines/brep/capabilities/brep-contract-v1',
      ]))

    const templates = await modernRequest('resources/templates/list')
    expect((templates.resourceTemplates as Array<{ uriTemplate: string }>))
      .toEqual(expect.arrayContaining([
        expect.objectContaining({
          uriTemplate: 'openscad://engines/{engine_class}/capabilities/{manifest_version}',
        }),
      ]))

    const listedEngines = await modernRequest('tools/call', {
      name: 'openscad_list_engines',
      arguments: {},
    }) as { structuredContent: Record<string, unknown> }
    const engineValidation = await fromJsonSchema(listEnginesTool!.outputSchema!)['~standard']
      .validate(listedEngines.structuredContent)
    expect(engineValidation).not.toHaveProperty('issues')
    const forgedRegistry = structuredClone(listedEngines.structuredContent) as {
      geometry_engines: { routes: Array<{ engine_class: string }> }
    }
    forgedRegistry.geometry_engines.routes[0].engine_class = 'brep'
    expect(await fromJsonSchema(listEnginesTool!.outputSchema!)['~standard']
      .validate(forgedRegistry)).toHaveProperty('issues')

    const analyzed = await modernRequest('tools/call', {
      name: 'openscad_analyze',
      arguments: { source: 'cube(1);', quality: 'full' },
    }) as { structuredContent: Record<string, unknown> }
    const analyzeValidator = fromJsonSchema(analyzeTool!.outputSchema!)['~standard']
    const analysisValidation = await analyzeValidator.validate(analyzed.structuredContent)
    expect(analysisValidation).not.toHaveProperty('issues')
    const mismatchedRoute = structuredClone(analyzed.structuredContent) as {
      analysis: { execution: { language_contract: string } }
    }
    mismatchedRoute.analysis.execution.language_contract = 'openscad-viewer/brep-1'
    expect(await analyzeValidator.validate(mismatchedRoute)).toHaveProperty('issues')
    const plannedSuccess = structuredClone(analyzed.structuredContent) as {
      analysis: { execution: { evidence: string } }
      build: { execution: { evidence: string } }
    }
    plannedSuccess.analysis.execution.evidence = 'planned'
    plannedSuccess.build.execution.evidence = 'planned'
    expect(await analyzeValidator.validate(plannedSuccess)).toHaveProperty('issues')

    const failedBrep = await modernRequest('tools/call', {
      name: 'openscad_analyze',
      arguments: {
        source: '// @language openscad-viewer/brep-1\ncube(1);',
        quality: 'full',
      },
    }) as { structuredContent: Record<string, unknown> }
    expect(await analyzeValidator.validate(failedBrep.structuredContent))
      .not.toHaveProperty('issues')
    const forgedFailure = structuredClone(failedBrep.structuredContent) as {
      build: {
        source_sha256: string
        execution: { automatic_fallback: boolean; engine_class: string }
      }
    }
    forgedFailure.build.source_sha256 = 'not-a-digest'
    forgedFailure.build.execution.automatic_fallback = true
    forgedFailure.build.execution.engine_class = 'manifold'
    expect(await analyzeValidator.validate(forgedFailure)).toHaveProperty('issues')

    const capabilities = await modernRequest('resources/read', { uri: 'openscad://capabilities' })
    expect(capabilities).toMatchObject({ ttlMs: 300_000, cacheScope: 'private' })
    expect(capabilities._meta).toMatchObject({
      'io.modelcontextprotocol/serverInfo': { name: 'open-scad-viewer', version: '0.1.0' },
    })
    const capabilityDocument = JSON.parse((capabilities.contents as Array<{ text: string }>)[0].text)
    expect(capabilityDocument).toMatchObject({
      geometry_host: {
        id: 'mcp-geometry-host-isolation-v1',
        boundary: 'worker-thread',
        provider_lifecycle: 'disposable-worker-per-job',
        limits: { job_deadline_ms: 30_000, max_concurrent_workers: 1 },
        residual_risks: { subprocess_boundary: false, os_enforced_memory_limit: false },
      },
    })

    await expect(request('tools/list')).rejects.toThrow(/protocolVersion|_meta|Invalid/i)
  })

  it('keeps cancelled work admitted until the actual tool handler settles', async () => {
    const geometry = new BlockingGeometryService()
    const {
      modernRequest,
      sendModernRequest,
      sendModernNotification,
      settledIds,
    } = await connectedModernServer({ geometry, maxInFlightRequests: 1 })
    await modernRequest('server/discover')

    await sendModernRequest(77, 'tools/call', {
      name: 'openscad_check',
      arguments: { source: 'cube(2);', quality: 'full' },
    })
    await geometry.started
    await sendModernNotification('notifications/cancelled', {
      requestId: 77,
      reason: 'test cancellation',
    })

    await expect(modernRequest('tools/list')).rejects.toThrow(/busy/i)
    geometry.release()
    await vi.waitFor(() => expect(settledIds.has(77)).toBe(true))

    const listed = await modernRequest('tools/list')
    expect(listed.tools).toEqual(expect.any(Array))
  })

  it('does not leak admission when cancellation races SDK validation or built-in ping', async () => {
    const {
      modernRequest,
      sendModernRequest,
      sendModernNotification,
      settledIds,
    } = await connectedModernServer({ maxInFlightRequests: 1 })
    await modernRequest('server/discover')

    const malformedCall = sendModernRequest(70, 'tools/call', { arguments: {} })
    const cancelMalformed = sendModernNotification('notifications/cancelled', {
      requestId: 70,
      reason: 'cancel before tool validation',
    })
    await Promise.all([malformedCall, cancelMalformed])
    await vi.waitFor(() => expect(settledIds.has(70)).toBe(true))
    await new Promise<void>(resolve => setTimeout(resolve, 0))
    expect((await modernRequest('tools/list')).tools).toEqual(expect.any(Array))
    await new Promise<void>(resolve => setTimeout(resolve, 0))

    const ping = sendModernRequest(71, 'ping')
    const cancelPing = sendModernNotification('notifications/cancelled', {
      requestId: 71,
      reason: 'cancel built-in handler',
    })
    await Promise.all([ping, cancelPing])
    await new Promise<void>(resolve => setTimeout(resolve, 0))
    expect((await modernRequest('tools/list')).tools).toEqual(expect.any(Array))
  })

  it('keeps lifecycle wrapping when modern stdio replaces the discover handler', async () => {
    const {
      modernRequest,
      sendModernRequest,
      sendModernNotification,
      settledIds,
    } = await connectedModernServer({ maxInFlightRequests: 1 })
    await modernRequest('server/discover')

    await Promise.all([
      sendModernRequest(72, 'server/discover'),
      sendModernNotification('notifications/cancelled', {
        requestId: 72,
        reason: 'cancel replaced discover handler',
      }),
    ])
    await vi.waitFor(() => expect(settledIds.has(72)).toBe(true))
    await new Promise<void>(resolve => setTimeout(resolve, 0))

    expect((await modernRequest('tools/list')).tools).toEqual(expect.any(Array))
  })

  it('settles an acknowledged subscription outside the request budget and remains usable after cancel', async () => {
    const {
      modernRequest,
      sendModernRequest,
      sendModernNotification,
      messages,
    } = await connectedModernServer({ maxInFlightRequests: 1, maxSubscriptions: 1 })
    await modernRequest('server/discover')

    await sendModernRequest(88, 'subscriptions/listen', {
      notifications: { resourcesListChanged: true },
    })
    await vi.waitFor(() => expect(messages).toContainEqual(expect.objectContaining({
      method: 'notifications/subscriptions/acknowledged',
      params: expect.objectContaining({
        _meta: expect.objectContaining({
          'io.modelcontextprotocol/subscriptionId': 88,
        }),
      }),
    })))
    await new Promise<void>(resolve => queueMicrotask(resolve))

    expect((await modernRequest('tools/list')).tools).toEqual(expect.any(Array))
    await sendModernNotification('notifications/cancelled', {
      requestId: 88,
      reason: 'close subscription',
    })
    expect((await modernRequest('tools/list')).tools).toEqual(expect.any(Array))
  })
})
