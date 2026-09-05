import { inflateSync } from 'node:zlib'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { InMemoryTransport, McpServer, type JSONRPCMessage } from '@modelcontextprotocol/server'
import { registerModelGraphNurbsTools } from '../src/mcp/modelGraphNurbsTools'
import { MODELGRAPH_NURBS_EXAMPLE, MODELGRAPH_NURBS_SURFACE_EXAMPLE } from '../src/services/modelGraphNurbs'

type RpcValue = Record<string, unknown>
type ToolResult = {
  isError?: boolean
  structuredContent: Record<string, unknown>
  content: Array<{ type: string; text?: string; data?: string; mimeType?: string; resource?: { uri: string; mimeType: string; blob: string } }>
}

describe('own NURBS through the MCP protocol', () => {
  let request: (method: string, params?: RpcValue) => Promise<RpcValue>
  let close: () => Promise<void>
  const tool = async (name: string, arguments_: RpcValue) => await request('tools/call', { name, arguments: arguments_ }) as unknown as ToolResult

  beforeAll(async () => {
    const server = new McpServer({ name: 'own-nurbs-test', version: '1.0.0' })
    registerModelGraphNurbsTools(server)
    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()
    const pending = new Map<number, (response: RpcValue) => void>()
    let nextId = 0
    clientTransport.onmessage = message => {
      if ('id' in message && typeof message.id === 'number' && ('result' in message || 'error' in message)) {
        pending.get(message.id)?.(message as RpcValue)
        pending.delete(message.id)
      }
    }
    await clientTransport.start()
    await server.connect(serverTransport)
    request = async (method, params = {}) => {
      const id = ++nextId
      const response = new Promise<RpcValue>((resolve, reject) => {
        const timer = setTimeout(() => { pending.delete(id); reject(new Error(`MCP timeout: ${method}`)) }, 10_000)
        pending.set(id, value => { clearTimeout(timer); resolve(value) })
      })
      await clientTransport.send({ jsonrpc: '2.0', id, method, params } as JSONRPCMessage)
      const envelope = await response
      if (envelope.error) throw new Error(JSON.stringify(envelope.error))
      return envelope.result as RpcValue
    }
    await request('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'own-nurbs-conformance', version: '1.0.0' } })
    await clientTransport.send({ jsonrpc: '2.0', method: 'notifications/initialized' } as JSONRPCMessage)
    close = async () => { await server.close(); await clientTransport.close() }
  })
  afterAll(async () => close?.())

  it('discovers the own-kernel tools and provides a language resource with precise export limits', async () => {
    const listed = await request('tools/list') as { tools: Array<{ name: string; annotations: { readOnlyHint: boolean } }> }
    expect(listed.tools.map(tool => tool.name).sort()).toEqual(['modelgraph_nurbs_build', 'modelgraph_nurbs_compile', 'modelgraph_nurbs_evaluate', 'modelgraph_nurbs_export', 'modelgraph_nurbs_language'])
    expect(listed.tools.every(tool => tool.annotations.readOnlyHint)).toBe(true)
    const resources = await request('resources/list') as { resources: Array<{ uri: string }> }
    expect(resources.resources.map(resource => resource.uri)).toContain('openscad://language/modelgraph-nurbs-1')
    const read = await request('resources/read', { uri: 'openscad://language/modelgraph-nurbs-1' }) as { contents: Array<{ text: string }> }
    const language = JSON.parse(read.contents[0].text)
    expect(language.language).toBe('modelgraph/nurbs-1')
    expect(language.guide).toContain('own TypeScript numerical kernel')
    expect(language.guide).toContain('no third-party spline or B-rep kernel')
    expect(language.guide).toMatch(/STEP.*not implemented/)
    expect(language.schema.properties.language.const).toBe('modelgraph/nurbs-1')
    const direct = await tool('modelgraph_nurbs_language', {})
    expect(direct.structuredContent).toEqual(language)
  })

  it('compiles parameterized control points and evaluates rational coordinates and derivatives', async () => {
    const compiled = await tool('modelgraph_nurbs_compile', { document: MODELGRAPH_NURBS_SURFACE_EXAMPLE })
    expect(compiled.isError).toBe(false)
    expect(compiled.structuredContent.execution_target).toBe('own-nurbs')
    const normalized = compiled.structuredContent.document as typeof MODELGRAPH_NURBS_SURFACE_EXAMPLE
    expect(JSON.stringify(normalized)).toContain('"param":"height"')
    expect(JSON.stringify(compiled.structuredContent.resolved_document)).not.toContain('"param":')

    const evaluated = await tool('modelgraph_nurbs_evaluate', { document: MODELGRAPH_NURBS_EXAMPLE, evaluations: [{ node: 'arc', u: 0.5 }] })
    expect(evaluated.isError).toBe(false)
    expect(evaluated.structuredContent.execution_target).toBe('own-nurbs')
    expect(evaluated.structuredContent.automatic_fallback).toBe(false)
    const value = (evaluated.structuredContent.evaluations as Array<{ point: number[]; d1: number[]; derivative_status: string }>)[0]
    expect(value.point[0]).toBeCloseTo(10 * Math.SQRT1_2, 10)
    expect(value.point[1]).toBeCloseTo(10 * Math.SQRT1_2, 10)
    expect(value.d1[0]).toBeCloseTo(-20 / (1 + Math.SQRT1_2), 10)
    expect(value.derivative_status).toBe('available')
  })

  it('builds a closed sampled solid and returns three actual PNG views and readable topology', async () => {
    const built = await tool('modelgraph_nurbs_build', { document: MODELGRAPH_NURBS_SURFACE_EXAMPLE })
    expect(built.isError).toBe(false)
    expect(built.structuredContent.ok).toBe(true)
    expect(built.structuredContent.execution_target).toBe('own-nurbs')
    expect(built.structuredContent.images_status).toBe('rendered')
    expect(built.structuredContent.views).toEqual(['front', 'top', 'isometric'])
    const report = built.structuredContent.report as { kernel: string; mesh: { closed: boolean; signedVolumeMm3: number; selfIntersectionStatus: string }; printability: string; error_bound_certified: boolean }
    expect(report.kernel).toBe('own-typescript-nurbs')
    expect(report.mesh.closed).toBe(true)
    expect(report.mesh.signedVolumeMm3).toBeCloseTo(800, 6)
    expect(report.mesh.selfIntersectionStatus).toBe('not_checked')
    expect(report.printability).toBe('unknown')
    expect(report.error_bound_certified).toBe(false)
    expect(built.structuredContent.mesh).toBeUndefined()
    const images = built.content.filter(item => item.type === 'image')
    expect(images).toHaveLength(3)
    for (const image of images) {
      expect(image.mimeType).toBe('image/png')
      const png = Buffer.from(image.data!, 'base64')
      expect(png.subarray(0, 8)).toEqual(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))
      expect(png.readUInt32BE(16)).toBe(192)
      expect(png.readUInt32BE(20)).toBe(192)
      const rows = inflateSync(png.subarray(41, 41 + png.readUInt32BE(33)))
      expect(rows.length).toBe(192 * 577)
      expect(rows.some(value => value !== 245 && value !== 0)).toBe(true)
    }
    expect(images[0].data).not.toBe(images[1].data)
  })

  it('exports a real sampled STL and preserves editable rational data in JSON', async () => {
    const stl = await tool('modelgraph_nurbs_export', { document: MODELGRAPH_NURBS_SURFACE_EXAMPLE, format: 'stl' })
    expect(stl.isError).toBe(false)
    const stlResource = stl.content.find(item => item.type === 'resource')!.resource!
    expect(stlResource.uri).toMatch(/^modelgraph:\/\/nurbs-export\/[a-f0-9]{64}\.stl$/)
    expect(stlResource.mimeType).toBe('model/stl')
    const stlText = Buffer.from(stlResource.blob, 'base64').toString('utf8')
    expect(stlText.startsWith('solid modelgraph_nurbs_sampled\n')).toBe(true)
    expect(stlText.match(/facet normal/g)!.length).toBeGreaterThan(100)
    expect(stlText).not.toMatch(/NaN|Infinity/)

    const json = await tool('modelgraph_nurbs_export', { document: MODELGRAPH_NURBS_SURFACE_EXAMPLE, format: 'json' })
    expect(json.isError).toBe(false)
    const jsonResource = json.content.find(item => item.type === 'resource')!.resource!
    expect(jsonResource.mimeType).toBe('application/json')
    const document = JSON.parse(Buffer.from(jsonResource.blob, 'base64').toString('utf8'))
    expect(document.language).toBe('modelgraph/nurbs-1')
    expect(document.nodes[0].control_points[1][1][2]).toEqual({ param: 'height' })
    expect(document.nodes[0].weights[1][1]).toBe(2)
    expect(document.parameters).toEqual(MODELGRAPH_NURBS_SURFACE_EXAMPLE.parameters)
  })

  it('returns kernel failures as errors and handles a subsequent valid request', async () => {
    const invalid = structuredClone(MODELGRAPH_NURBS_EXAMPLE)
    if (invalid.nodes[0].op === 'curve') invalid.nodes[0].weights[1] = 0
    const failed = await tool('modelgraph_nurbs_build', { document: invalid })
    expect(failed.isError).toBe(true)
    expect(failed.structuredContent.ok).toBe(false)
    expect(failed.structuredContent.error).toMatchObject({ code: 'NURBS_OPERATION_FAILED' })
    expect(failed.content.some(item => item.type === 'image')).toBe(false)
    const openStl = await tool('modelgraph_nurbs_export', { document: MODELGRAPH_NURBS_EXAMPLE, format: 'stl' })
    expect(openStl.isError).toBe(true)
    expect(openStl.content.some(item => item.type === 'resource')).toBe(false)
    const recovered = await tool('modelgraph_nurbs_evaluate', { document: MODELGRAPH_NURBS_EXAMPLE, evaluations: [{ node: 'arc', u: 0 }] })
    expect(recovered.isError).toBe(false)
    expect((recovered.structuredContent.evaluations as Array<{ point: number[] }>)[0].point).toEqual([10, 0, 0])
  })

  it('rejects unsupported STEP export rather than returning a mislabeled artifact', async () => {
    const result = await tool('modelgraph_nurbs_export', { document: MODELGRAPH_NURBS_SURFACE_EXAMPLE, format: 'step' })
    expect(result.isError).toBe(true)
    expect(result.content.some(item => item.type === 'resource')).toBe(false)
  })
  it('transports a binary 3MF package through the actual own-kernel process', async () => {
    const result = await tool('modelgraph_nurbs_export', { document: MODELGRAPH_NURBS_SURFACE_EXAMPLE, format: '3mf' })
    expect(result.isError).toBe(false)
    const resource = result.content.find((c: any) => c.type === 'resource') as any
    expect(resource.resource.uri).toMatch(/\.3mf$/)
    expect(Buffer.from(resource.resource.blob, 'base64').readUInt32LE(0)).toBe(0x04034b50)
  })

})
