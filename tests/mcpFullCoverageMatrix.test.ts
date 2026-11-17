import { createServer } from 'node:net'
import Ajv2020 from 'ajv/dist/2020'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { InMemoryTransport, type JSONRPCMessage } from '@modelcontextprotocol/server'
import { exportDirectIgesV2 } from '../src/kernel/cadIges'
import { createBrepBox } from '../src/kernel/geometry/brep'
import { warmGeometryKernel } from '../src/kernel/geometry/kernel'
import { createRushGraphMcpServer } from '../src/mcp/createRushGraphServer'
import { DuckDbModelStore } from '../src/mcp/duckdbModelStore'
import {
  buildToolFixtures,
  buildTemplateUris,
  createMockOfficialRuntime,
  PROMPT_ARGUMENTS,
  type CoverageSeedIds,
} from './helpers/mcpCoverageFixtures'

interface JsonRpcResponse {
  jsonrpc: '2.0'
  id: number
  result?: Record<string, unknown>
  error?: { code: number; message: string }
}

const ajv = new Ajv2020({ strict: false, allErrors: true })

async function freeLoopbackPort(): Promise<number> {
  const srv = createServer()
  await new Promise<void>(resolve => srv.listen(0, '127.0.0.1', resolve))
  const port = (srv.address() as { port: number }).port
  await new Promise<void>(resolve => srv.close(() => resolve()))
  return port
}

describe('MCP full-coverage matrix (100% tools, resources, templates, prompts)', () => {
  let store: DuckDbModelStore
  let server: ReturnType<typeof createRushGraphMcpServer>
  let clientTransport: InMemoryTransport
  let request: (method: string, params?: Record<string, unknown>, allowRpcError?: boolean) => Promise<JsonRpcResponse>
  let seedIds: CoverageSeedIds

  beforeAll(async () => {
    await warmGeometryKernel()
    store = await DuckDbModelStore.open(':memory:')
    server = createRushGraphMcpServer({
      store,
      officialRuntime: createMockOfficialRuntime(),
      localEditor: true,
    })
    const [client, srv] = InMemoryTransport.createLinkedPair()
    clientTransport = client
    const pending = new Map<number, (response: JsonRpcResponse) => void>()
    clientTransport.onmessage = message => {
      if ('id' in message && typeof message.id === 'number' && ('result' in message || 'error' in message)) {
        pending.get(message.id)?.(message as JsonRpcResponse)
        pending.delete(message.id)
      }
    }
    await clientTransport.start()
    await server.connect(srv)
    let nextId = 1
    request = async (method, params = {}, allowRpcError = false) => {
      const id = nextId++
      const promise = new Promise<JsonRpcResponse>((resolve, reject) => {
        const timeout = setTimeout(() => {
          pending.delete(id)
          reject(new Error(`Timed out waiting for ${method}`))
        }, 30_000)
        pending.set(id, value => {
          clearTimeout(timeout)
          resolve(value)
        })
      })
      await clientTransport.send({ jsonrpc: '2.0', id, method, params } as JSONRPCMessage)
      const response = await promise
      if (!allowRpcError && response.error) throw new Error(`${response.error.code}: ${response.error.message}`)
      return response
    }

    await request('initialize', {
      protocolVersion: '2025-11-25',
      capabilities: {},
      clientInfo: { name: 'full-matrix', version: '1.0.0' },
    })
    await clientTransport.send({ jsonrpc: '2.0', method: 'notifications/initialized' } as JSONRPCMessage)

    const callSeed = async (name: string, args: Record<string, unknown>) => {
      const res = await request('tools/call', { name, arguments: args })
      const sc = res.result?.structuredContent as Record<string, unknown>
      if (res.result?.isError) throw new Error(`Seed call ${name} failed: ${JSON.stringify(sc)}`)
      return sc
    }

    await callSeed('openscad_save_model', { id: 'seed:part', name: 'Seed', source: 'w = 4;\ncube([w, 4, 4]);' })
    await callSeed('openscad_save_model', {
      id: 'seed:customizable',
      name: 'Custom',
      source: 'w = 4;\ncube([w, 4, 4]);',
    })
    await callSeed('openscad_save_model', { id: 'seed:disposable', name: 'Disposable', source: 'cube(1);' })
    const analyzed = await callSeed('openscad_analyze', { model_id: 'seed:part', quality: 'full' })
    const exported = await callSeed('openscad_export', { model_id: 'seed:part', format: 'stl' })
    const indExport = await callSeed('openscad_independent_export', { source: 'cube(2);', format: 'stl' })
    const offExport = await callSeed('openscad_official_export', { source: 'cube(2);', format: 'stl' })

    seedIds = {
      buildId: String((analyzed.build as { id: string }).id),
      artifactId: String((exported.artifact as { id: string }).id),
      independentArtifactUri: String((indExport.artifact as { resource_uri: string }).resource_uri),
      officialArtifactUri: String((offExport.artifact as { resource_uri: string }).resource_uri),
      closedEditorPort: await freeLoopbackPort(),
    }
  }, 45_000)

  afterAll(async () => {
    await server?.close()
    await clientTransport?.close()
    await store?.close()
  })

  it('covers 100% of tools with Ajv outputSchema validation, invalid-args rejection, and readOnlyHint immutability', async () => {
    const listed = ((await request('tools/list')).result?.tools ?? []) as Array<{
      name: string
      inputSchema?: { properties?: Record<string, unknown>; additionalProperties?: unknown }
      outputSchema?: Record<string, unknown>
      annotations?: { readOnlyHint?: boolean }
    }>
    const fixtures = buildToolFixtures(seedIds)
    fixtures.cad_import_iges = {
      validArgs: { iges_text: exportDirectIgesV2(createBrepBox([0, 0, 0], [3, 3, 3])).text },
      invalidArgs: fixtures.cad_import_iges.invalidArgs,
    }

    const unmapped = listed.map(t => t.name).filter(name => !(name in fixtures))
    expect(unmapped, 'Every registered tool must have a matrix fixture').toEqual([])

    for (const tool of listed) {
      const fixture = fixtures[tool.name]!
      const beforeModels = await store.listModels(500)
      const beforeBuilds = await store.listBuilds({ limit: 500 })

      const validRes = await request('tools/call', { name: tool.name, arguments: fixture.validArgs })
      const validResult = validRes.result as { isError?: boolean; structuredContent?: Record<string, unknown> }
      expect(validResult.isError, `${tool.name} validArgs returned isError: ${JSON.stringify(validResult)}`).not.toBe(
        true,
      )

      if (tool.annotations?.readOnlyHint === true) {
        expect(await store.listModels(500), `${tool.name} readOnlyHint models mutation`).toEqual(beforeModels)
        expect(await store.listBuilds({ limit: 500 }), `${tool.name} readOnlyHint builds mutation`).toEqual(
          beforeBuilds,
        )
      }

      if (tool.outputSchema) {
        const validate = ajv.compile(tool.outputSchema)
        const ok = validate(validResult.structuredContent)
        expect(ok, `${tool.name} outputSchema mismatch: ${ajv.errorsText(validate.errors)}`).toBe(true)
      }

      const hasProperties = Object.keys(tool.inputSchema?.properties ?? {}).length > 0
      const canRejectExtra = hasProperties || tool.inputSchema?.additionalProperties === false
      if (canRejectExtra) {
        const invalidRes = await request('tools/call', { name: tool.name, arguments: fixture.invalidArgs }, true)
        const invalidRejected =
          Boolean(invalidRes.error) || (invalidRes.result as { isError?: boolean } | undefined)?.isError === true
        expect(invalidRejected, `${tool.name} invalidArgs must fail cleanly`).toBe(true)
      }
    }
  }, 60_000)

  it('covers 100% of static resources, resource templates, and prompts', async () => {
    const resources = ((await request('resources/list')).result?.resources ?? []) as Array<{ uri: string }>
    expect(resources.length).toBeGreaterThanOrEqual(9)
    for (const resource of resources) {
      const read = await request('resources/read', { uri: resource.uri })
      const contents = read.result?.contents as Array<{ text?: string; blob?: string }>
      expect(contents.length, `Resource ${resource.uri} empty`).toBeGreaterThan(0)
      expect(Boolean(contents[0]?.text || contents[0]?.blob), `Resource ${resource.uri} blank`).toBe(true)
    }

    const templates = ((await request('resources/templates/list')).result?.resourceTemplates ?? []) as Array<{
      uriTemplate: string
    }>
    const templateUris = buildTemplateUris(seedIds)
    const missingTemplates = templates.map(t => t.uriTemplate).filter(uri => !(uri in templateUris))
    expect(missingTemplates, 'Every resource template must have a concrete read URI').toEqual([])
    for (const tmpl of templates) {
      const read = await request('resources/read', { uri: templateUris[tmpl.uriTemplate]! })
      const contents = read.result?.contents as Array<{ text?: string; blob?: string }>
      expect(contents.length, `Template ${tmpl.uriTemplate} empty`).toBeGreaterThan(0)
    }

    const prompts = ((await request('prompts/list')).result?.prompts ?? []) as Array<{ name: string }>
    const missingPrompts = prompts.map(p => p.name).filter(name => !(name in PROMPT_ARGUMENTS))
    expect(missingPrompts, 'Every prompt must have matrix arguments').toEqual([])
    for (const prompt of prompts) {
      const res = await request('prompts/get', { name: prompt.name, arguments: PROMPT_ARGUMENTS[prompt.name]! })
      const messages = res.result?.messages as Array<{ content: { text?: string } }>
      expect(messages.length, `Prompt ${prompt.name} empty`).toBeGreaterThan(0)
      expect(messages[0]?.content.text?.length ?? 0, `Prompt ${prompt.name} text empty`).toBeGreaterThan(10)
    }
  })
})
