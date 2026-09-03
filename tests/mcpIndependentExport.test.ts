import { createHash } from 'node:crypto'
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  fromJsonSchema,
  InMemoryTransport,
  type JSONRPCMessage,
} from '@modelcontextprotocol/server'
import { createOpenScadMcpServer } from '../src/mcp/createServer'
import { DuckDbModelStore } from '../src/mcp/duckdbModelStore'
import { executeIndependentOpenScad } from '../src/mcp/independentOpenScadExecution'
import type { OfficialOpenScadRuntimeService } from '../src/mcp/officialOpenScadRuntimeService'

interface Connection {
  request(method: string, params?: Record<string, unknown>): Promise<Record<string, unknown>>
  close(): Promise<void>
  store: DuckDbModelStore
}

const connections: Connection[] = []

afterEach(async () => {
  await Promise.all(connections.splice(0).map(connection => connection.close()))
})

async function connect(): Promise<Connection & {
  upstream: {
    capabilities: ReturnType<typeof vi.fn>
    check: ReturnType<typeof vi.fn>
    export: ReturnType<typeof vi.fn>
  }
}> {
  const store = await DuckDbModelStore.open(':memory:')
  const upstream = {
    capabilities: vi.fn(async () => { throw new Error('upstream oracle must not be called') }),
    check: vi.fn(async () => { throw new Error('upstream oracle must not be called') }),
    export: vi.fn(async () => { throw new Error('upstream oracle must not be called') }),
    close: vi.fn(async () => undefined),
  }
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()
  const server = createOpenScadMcpServer({
    store,
    officialRuntime: upstream as unknown as OfficialOpenScadRuntimeService,
  })
  const pending = new Map<number, (value: Record<string, unknown>) => void>()
  let nextId = 1
  clientTransport.onmessage = message => {
    if (!('id' in message) || typeof message.id !== 'number'
      || (!('result' in message) && !('error' in message))) return
    pending.get(message.id)?.(message as Record<string, unknown>)
    pending.delete(message.id)
  }
  await clientTransport.start()
  await server.connect(serverTransport)
  const request = async (method: string, params: Record<string, unknown> = {}) => {
    const id = nextId++
    const response = new Promise<Record<string, unknown>>((resolve, reject) => {
      const timeout = setTimeout(() => {
        pending.delete(id)
        reject(new Error(`Timed out waiting for ${method}`))
      }, 15_000)
      pending.set(id, value => {
        clearTimeout(timeout)
        resolve(value)
      })
    })
    await clientTransport.send({ jsonrpc: '2.0', id, method, params } as JSONRPCMessage)
    const envelope = await response
    if (envelope.error) throw new Error(JSON.stringify(envelope.error))
    return (envelope.result ?? {}) as Record<string, unknown>
  }
  await request('initialize', {
    protocolVersion: '2025-11-25',
    capabilities: {},
    clientInfo: { name: 'independent-export-test', version: '1.0.0' },
  })
  await clientTransport.send({ jsonrpc: '2.0', method: 'notifications/initialized' } as JSONRPCMessage)
  const connection = {
    store,
    upstream,
    request,
    close: async () => {
      await server.close()
      await clientTransport.close()
      await store.close()
    },
  }
  connections.push(connection)
  return connection
}

describe('independent OpenSCAD export through MCP', () => {
  it('lists, exports and serves exact full-quality STL bytes without upstream or DuckDB writes', async () => {
    const { request, upstream, store } = await connect()
    const listed = await request('tools/list') as {
      tools: Array<{ name: string; outputSchema: Record<string, unknown> }>
    }
    const tool = listed.tools.find(candidate => candidate.name === 'openscad_independent_export')
    expect(tool).toBeDefined()

    const called = await request('tools/call', {
      name: 'openscad_independent_export',
      arguments: {
        source: 'include <size.scad> cube(edge);',
        files: [{ path: 'size.scad', text: 'edge = 2;' }],
        format: 'stl',
        file_name: 'bounded-part',
        max_bytes: 4096,
      },
    }) as {
      isError?: boolean
      content: Array<Record<string, unknown>>
      structuredContent: {
        export: {
          engine: Record<string, unknown>
          quality: string
          format: string
          metrics: { mesh_count: number; triangle_count: number; volume: number }
          warnings: string[]
        }
        artifact: {
          id: string
          resource_uri: string
          file_name: string
          mime_type: string
          sha256: string
          byte_length: number
        }
      }
    }
    expect(called.isError).not.toBe(true)
    expect(await fromJsonSchema(tool!.outputSchema)['~standard']
      .validate(called.structuredContent)).not.toHaveProperty('issues')
    expect(called.structuredContent.export).toMatchObject({
      engine: {
        id: 'open-scad-viewer/independent-2021.01-dev.1',
        stage: 'development',
        upstream_runtime_used: false,
        production_authoritative: false,
        complete_language_claim: false,
      },
      quality: 'full',
      format: 'stl',
      metrics: { mesh_count: 1, triangle_count: 12, volume: 8 },
      warnings: [],
    })
    expect(called.structuredContent.export).not.toHaveProperty('provider_role')
    expect(called.structuredContent).not.toHaveProperty('build')
    expect(called.structuredContent.artifact).toMatchObject({
      id: expect.stringMatching(/^[a-f0-9]{64}$/),
      resource_uri: expect.stringMatching(/^openscad:\/\/independent-artifacts\/[a-f0-9]{64}$/),
      file_name: 'bounded-part.stl',
      mime_type: 'model/stl',
      sha256: expect.stringMatching(/^[a-f0-9]{64}$/),
      byte_length: 684,
    })
    expect(called.content).toContainEqual(expect.objectContaining({
      type: 'resource_link',
      uri: called.structuredContent.artifact.resource_uri,
      name: 'bounded-part.stl',
      mimeType: 'model/stl',
    }))

    const resource = await request('resources/read', {
      uri: called.structuredContent.artifact.resource_uri,
    }) as { contents: Array<{ blob: string; mimeType: string }> }
    const bytes = Buffer.from(resource.contents[0].blob, 'base64')
    const byteDigest = createHash('sha256').update(bytes).digest('hex')
    expect(resource.contents[0].mimeType).toBe('model/stl')
    expect(bytes.byteLength).toBe(called.structuredContent.artifact.byte_length)
    expect(byteDigest).toBe(called.structuredContent.artifact.sha256)
    expect(byteDigest).toBe(called.structuredContent.artifact.id)

    const resources = await request('resources/list') as { resources: Array<{ uri: string }> }
    expect(resources.resources).toContainEqual(expect.objectContaining({
      uri: called.structuredContent.artifact.resource_uri,
    }))
    const templates = await request('resources/templates/list') as {
      resourceTemplates: Array<{ uriTemplate: string }>
    }
    expect(templates.resourceTemplates).toContainEqual(expect.objectContaining({
      uriTemplate: 'openscad://independent-artifacts/{sha256}',
    }))

    const stats = await store.getCatalogStats()
    expect(stats.buildCount).toBe(0)
    expect(stats.artifactCount).toBe(0)
    expect(upstream.capabilities).not.toHaveBeenCalled()
    expect(upstream.check).not.toHaveBeenCalled()
    expect(upstream.export).not.toHaveBeenCalled()
  })

  it('serves exact OBJ text and rejects max_bytes before publishing another artifact', async () => {
    const { request } = await connect()
    const exported = await request('tools/call', {
      name: 'openscad_independent_export',
      arguments: { source: 'cube(1);', format: 'obj', file_name: 'cube.obj' },
    }) as {
      structuredContent: { artifact: { resource_uri: string; id: string; sha256: string; byte_length: number } }
    }
    const resource = await request('resources/read', {
      uri: exported.structuredContent.artifact.resource_uri,
    }) as { contents: Array<{ text: string; mimeType: string }> }
    const bytes = new TextEncoder().encode(resource.contents[0].text)
    expect(resource.contents[0].mimeType).toBe('model/obj')
    expect(createHash('sha256').update(bytes).digest('hex')).toBe(exported.structuredContent.artifact.id)
    expect(exported.structuredContent.artifact.sha256).toBe(exported.structuredContent.artifact.id)
    expect(bytes.byteLength).toBe(exported.structuredContent.artifact.byte_length)

    const rejected = await request('tools/call', {
      name: 'openscad_independent_export',
      arguments: { source: 'cube(1);', format: 'stl', max_bytes: 100 },
    }) as { isError?: boolean; structuredContent?: { error?: { code?: string } } }
    expect(rejected).toMatchObject({
      isError: true,
      structuredContent: { error: { code: 'artifact_too_large' } },
    })
  })

  it('honors an already-aborted signal before entering the parser', async () => {
    const controller = new AbortController()
    controller.abort()
    await expect(executeIndependentOpenScad({
      source: 'cube(1);',
      files: [],
      quality: 'full',
      time: 0,
      signal: controller.signal,
    })).rejects.toMatchObject({ name: 'AbortError' })
  })
})
