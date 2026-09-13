import { afterEach, describe, expect, it } from 'vitest'
import { InMemoryTransport, McpServer, type JSONRPCMessage } from '@modelcontextprotocol/server'
import { registerModelGraphSvgTools } from '../src/mcp/modelGraphSvgTools'
import { HeadlessGeometryService, type GeometryAnalysis } from '../src/mcp/geometryService'
import { setModelGraphParameters, type ModelGraph } from '../src/services/modelGraph'

interface ToolResult {
  isError?: boolean
  content: Array<{ type: 'text'; text: string } | { type: 'resource'; resource: { uri: string; mimeType: string; text: string } }>
}
interface JsonRpcResponse {
  id: number
  result?: Record<string, unknown>
  error?: { code: number; message: string }
}
interface ToolSchema {
  name: string
  annotations: { readOnlyHint: boolean; openWorldHint: boolean }
  inputSchema: { type: string; required: string[]; additionalProperties: boolean; properties: Record<string, Record<string, unknown>> }
}
interface ExtrusionResult { source: string; svg: string; warnings: string[]; analysis: GeometryAnalysis }
interface ModelGraphExtrusionResult extends ExtrusionResult { document: ModelGraph; document_sha256: string; modelgraph_source: string; modelgraph_analysis: GeometryAnalysis }
const closeables: Array<() => Promise<void>> = []
afterEach(async () => { await Promise.all(closeables.splice(0).map(close => close())) })

/** Exercise the public MCP boundary without loading storage, other tools, or an upstream runtime. */
async function connect() {
  const server = new McpServer({ name: 'svg-workflow-test', version: '1.0.0' })
  registerModelGraphSvgTools(server, new HeadlessGeometryService())
  const [client, transport] = InMemoryTransport.createLinkedPair()
  const pending = new Map<number, (response: JsonRpcResponse) => void>()
  let nextId = 1
  client.onmessage = message => {
    if ('id' in message && typeof message.id === 'number' && ('result' in message || 'error' in message)) {
      pending.get(message.id)?.(message as JsonRpcResponse)
      pending.delete(message.id)
    }
  }
  await client.start()
  await server.connect(transport)
  closeables.push(async () => { await server.close(); await client.close() })
  async function request(method: string, params: Record<string, unknown> = {}) {
    const id = nextId++
    const response = new Promise<JsonRpcResponse>((resolve, reject) => {
      const timeout = setTimeout(() => { pending.delete(id); reject(new Error(`Timed out waiting for ${method}`)) }, 15000)
      pending.set(id, result => { clearTimeout(timeout); resolve(result) })
    })
    await client.send({ jsonrpc: '2.0', id, method, params } as JSONRPCMessage)
    const envelope = await response
    if (envelope.error) throw new Error(`${envelope.error.code}: ${envelope.error.message}`)
    return envelope.result ?? {}
  }
  await request('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'svg-test', version: '1.0.0' } })
  await client.send({ jsonrpc: '2.0', method: 'notifications/initialized' } as JSONRPCMessage)
  return { request, call: async (name: string, args: Record<string, unknown>) => await request('tools/call', { name, arguments: args }) as unknown as ToolResult }
}
function text(result: ToolResult) {
  const item = result.content.find(item => item.type === 'text')
  expect(item).toBeDefined()
  return item!.type === 'text' ? item!.text : ''
}
function resource(result: ToolResult) {
  const item = result.content.find(item => item.type === 'resource')
  expect(item, JSON.stringify(result)).toBeDefined()
  if (!item || item.type !== 'resource') throw new Error('Expected an SVG resource')
  return item.resource
}
function extrusion(result: ToolResult): ExtrusionResult {
  expect(result.isError, text(result)).not.toBe(true)
  return JSON.parse(text(result)) as ExtrusionResult
}
const rectangle = '<svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="10mm" viewBox="0 0 10 10"><rect width="10" height="10"/></svg>'
const styled = `<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="10mm" viewBox="0 0 20 10">
  <defs><rect id="tile" width="8" height="8"/><clipPath id="window"><rect x="1" y="2" width="14" height="4"/></clipPath></defs>
  <style>.ink { fill: #2563eb; } .hidden { display: none; }</style>
  <g class="ink" clip-path="url(#window)"><use href="#tile"/><use href="#tile" x="10"/></g>
  <rect class="hidden" width="20" height="10"/>
</svg>`

describe('SVG tools over MCP', () => {
  it('advertises all three tools with strict typed controls and the standalone 96 DPI default', async () => {
    const { request } = await connect()
    const listed = await request('tools/list') as unknown as { tools: ToolSchema[] }
    expect(listed.tools.map(tool => tool.name).sort()).toEqual(['modelgraph_svg_export', 'modelgraph_svg_extrude', 'modelgraph_svg_preview'])
    for (const name of ['modelgraph_svg_preview', 'modelgraph_svg_extrude']) {
      const tool = listed.tools.find(tool => tool.name === name)!
      expect(tool.annotations).toMatchObject({ readOnlyHint: true, openWorldHint: false })
      expect(tool.inputSchema).toMatchObject({ type: 'object', additionalProperties: false })
      expect(tool.inputSchema.required).toContain('svg')
      expect(tool.inputSchema.properties).toMatchObject({
        svg: { type: 'string', maxLength: 4 * 1024 * 1024 },
        dpi: { type: 'number', default: 96 },
        tolerance: { type: 'number', minimum: 0.0001, maximum: 10, default: 0.02 },
        geometryMode: { enum: ['vector', 'silhouette'], default: 'vector' },
        rasterSize: { type: 'integer', minimum: 128, maximum: 2048, default: 512 },
        alphaThreshold: { type: 'number', minimum: 0.01, maximum: 1, default: 0.5 },
        fonts: { type: 'array', maxItems: 16 },
      })
    }
    expect(listed.tools.find(tool => tool.name === 'modelgraph_svg_extrude')!.inputSchema.required).toContain('height')
    expect(listed.tools.find(tool => tool.name === 'modelgraph_svg_extrude')!.inputSchema.properties.includeModelGraph).toMatchObject({ type: 'boolean', default: false })
    expect(listed.tools.find(tool => tool.name === 'modelgraph_svg_export')!.inputSchema.properties.axis).toMatchObject({ enum: ['x', 'y', 'z'], default: 'z' })
  })

  it('returns metadata and portable artwork retaining gradients and outlined text', async () => {
    const { call } = await connect()
    const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="60mm" height="20mm" viewBox="0 0 60 20"><defs><linearGradient id="ink"><stop stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient></defs><rect x="1" y="1" width="18" height="18" fill="url(#ink)"/><text x="24" y="15" font-family="Noto Sans" font-size="12" fill="#008000">SVG</text></svg>'
    const result = await call('modelgraph_svg_preview', { svg })
    expect(result.isError, text(result)).not.toBe(true)
    expect(result.content.map(item => item.type)).toEqual(['text', 'resource'])
    const metadata = JSON.parse(text(result))
    expect(metadata).toMatchObject({ widthMm: 60, heightMm: 20, warnings: expect.any(Array) })
    const output = resource(result)
    expect(output).toMatchObject({ uri: 'modelgraph://svg/preview.svg', mimeType: 'image/svg+xml' })
    expect(output.text).toContain('<linearGradient')
    expect(output.text).toMatch(/url\(#/)
    expect(output.text).not.toContain('<text')
    expect((output.text.match(/<path\b/g) ?? []).length).toBeGreaterThan(1)
    expect(metadata.warnings.join(' ')).toMatch(/text.*outlines/i)
    const model = extrusion(await call('modelgraph_svg_extrude', { svg: output.text, height: 1 }))
    expect(model.analysis.volume).toBeGreaterThan(324)
    expect(model.analysis.topology).toMatchObject({ boundary: 0, nonManifold: 0 })
  })

  it('builds CSS, use and clipping into the expected solid and roundtrips contour SVG at physical scale', async () => {
    const { call } = await connect()
    // Clipped tiles have areas 7×4 and 5×4 mm²; the hidden rectangle adds no material.
    const model = extrusion(await call('modelgraph_svg_extrude', { svg: styled, height: 3, tolerance: 0.005 }))
    expect(model.analysis.volume).toBeCloseTo(144, 5)
    expect(model.analysis.topology).toMatchObject({ boundary: 0, nonManifold: 0 })
    expect(model.source).toContain('linear_extrude(height=3)')
    expect(model.source).not.toContain('import(')
    expect(model.svg).toContain('mm"')
    expect(model.svg).toContain('fill-rule="evenodd"')
    const restored = extrusion(await call('modelgraph_svg_extrude', { svg: model.svg, height: 3, dpi: 300 }))
    expect(restored.analysis.volume).toBeCloseTo(144, 5)
    // A tight SVG viewBox resets the origin; fabrication dimensions and area remain unchanged.
    const dimensions = (analysis: GeometryAnalysis) => analysis.bounds!.max.map((value, axis) => value - analysis.bounds!.min[axis])
    expect(dimensions(restored.analysis)).toEqual([14, 4, 3])
    expect(dimensions(restored.analysis)).toEqual(dimensions(model.analysis))
  })

  it('applies explicit pixel DPI to preview and extrusion while preserving millimeter dimensions', async () => {
    const { call } = await connect()
    const pixels = '<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96"><rect width="96" height="96"/></svg>'
    for (const [dpi, size] of [[96, 25.4], [192, 12.7]]) {
      const preview = await call('modelgraph_svg_preview', { svg: pixels, dpi })
      expect(preview.isError, text(preview)).not.toBe(true)
      expect(JSON.parse(text(preview)).widthMm).toBeCloseTo(size, 5)
      const model = extrusion(await call('modelgraph_svg_extrude', { svg: pixels, dpi, height: 1 }))
      expect(model.analysis.volume).toBeCloseTo(size * size, 3)
    }
  })

  it('returns an editable native ModelGraph preserving holes, islands, separate parts and physical coordinates', async () => {
    const { call } = await connect()
    const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="30mm" height="20mm" viewBox="0 0 30 20"><path fill-rule="evenodd" d="M0 0H20V20H0Z M5 5H15V15H5Z M9 9H11V11H9Z"/><rect x="24" y="2" width="4" height="3"/></svg>'
    const result = extrusion(await call('modelgraph_svg_extrude', { svg, height: 2, includeModelGraph: true })) as ModelGraphExtrusionResult
    const area = 20 * 20 - 10 * 10 + 2 * 2 + 4 * 3
    expect(result.document).toMatchObject({ language: 'modelgraph/1', units: 'mm', parameters: [{ id: 'svg_height', value: 2 }] })
    expect(result.document.nodes.filter(node => node.op === 'polygon')).toHaveLength(4)
    expect(result.document.nodes.filter(node => node.op === 'difference')).toHaveLength(2)
    expect(result.document_sha256).toMatch(/^[a-f0-9]{64}$/)
    expect(result.analysis.volume).toBeCloseTo(area * 2, 5)
    expect(result.modelgraph_analysis.volume).toBeCloseTo(area * 2, 5)
    expect(result.modelgraph_analysis.bounds).toEqual(result.analysis.bounds)
    expect(result.modelgraph_analysis.topology).toMatchObject({ boundary: 0, nonManifold: 0 })
    expect(result.modelgraph_source).not.toContain('import(')
    const edited = setModelGraphParameters(result.document, result.document_sha256, [{ id: 'svg_height', value: 3 }])
    const analysis = await new HeadlessGeometryService().analyze(edited.source, 'full')
    expect(analysis.volume).toBeCloseTo(area * 3, 5)
    const exported = await call('modelgraph_svg_export', { document: edited.document, axis: 'z' })
    expect(exported.isError, JSON.stringify(exported)).not.toBe(true)
    const restored = extrusion(await call('modelgraph_svg_extrude', { svg: resource(exported).text, height: 3 }))
    expect(restored.analysis.volume).toBeCloseTo(area * 3, 5)
    expect(restored).not.toHaveProperty('document')
  })

  it('rejects a native graph over the node limit without losing the SCAD-only workflow or connection', async () => {
    const { call } = await connect()
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="256mm" height="2mm" viewBox="0 0 256 2">${Array.from({ length: 128 }, (_, index) => `<rect x="${index * 2}" width="1" height="1"/>`).join('')}</svg>`
    const limited = await call('modelgraph_svg_extrude', { svg, height: 1, includeModelGraph: true })
    expect(limited.isError).toBe(true)
    expect(text(limited)).toMatch(/ModelGraph.*128 nodes/i)
    expect(limited.content.every(item => item.type === 'text')).toBe(true)
    const model = extrusion(await call('modelgraph_svg_extrude', { svg, height: 1 }))
    expect(model.analysis.volume).toBeCloseTo(128, 5)
    expect(model).not.toHaveProperty('document')
  })

  it('reports the native polygon point limit instead of reducing curve detail', async () => {
    const { call } = await connect()
    const points = Array.from({ length: 257 }, (_, index) => {
      const angle = index * Math.PI * 2 / 257
      return `${10 + Math.cos(angle) * 9},${10 + Math.sin(angle) * 9}`
    }).join(' ')
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><polygon points="${points}"/></svg>`
    const limited = await call('modelgraph_svg_extrude', { svg, height: 1, includeModelGraph: true })
    expect(limited.isError).toBe(true)
    expect(text(limited)).toMatch(/ModelGraph.*256 points/i)
    const model = extrusion(await call('modelgraph_svg_extrude', { svg, height: 1 }))
    const area = 257 * 9 ** 2 * Math.sin(2 * Math.PI / 257) / 2
    expect(model.analysis.volume).toBeCloseTo(area, 4)
  })

  it('requires explicit silhouette conversion for masks and reports the requested raster settings', async () => {
    const { call } = await connect()
    const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="16mm" height="16mm" viewBox="0 0 16 16"><defs><mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="16" height="16"><rect width="8" height="16" fill="white"/><rect x="8" width="8" height="16" fill="black"/></mask></defs><rect width="16" height="16" mask="url(#m)"/></svg>'
    const vector = await call('modelgraph_svg_extrude', { svg, height: 2 })
    expect(vector.isError).toBe(true)
    expect(text(vector)).toMatch(/silhouette/i)
    const rendered = extrusion(await call('modelgraph_svg_extrude', { svg, height: 2, geometryMode: 'silhouette', rasterSize: 128, alphaThreshold: 0.75 }))
    expect(rendered.analysis.volume).toBeCloseTo(256, 4)
    expect(rendered.warnings.join(' ')).toMatch(/128/)
    expect(rendered.warnings.join(' ')).toMatch(/0\.75/)
    expect(rendered.warnings.join(' ')).toMatch(/approximat/i)
  })

  it('returns tool-level typed validation failures for invalid parameters without breaking the connection', async () => {
    const { call } = await connect()
    for (const invalid of [{ dpi: '96' }, { geometryMode: 'automatic' }, { rasterSize: 127 }, { rasterSize: 256.5 }, { alphaThreshold: 0 }, { tolerance: 0 }, { unexpected: true }]) {
      const result = await call('modelgraph_svg_preview', { svg: rectangle, ...invalid })
      expect(result.isError, JSON.stringify(invalid)).toBe(true)
      expect(text(result)).toContain('Input validation error')
      expect(result.content.every(item => item.type === 'text')).toBe(true)
    }
    const invalidHeight = await call('modelgraph_svg_extrude', { svg: rectangle, height: -1 })
    expect(invalidHeight.isError).toBe(true)
    expect(text(invalidHeight)).toContain('Input validation error')
    const recovery = await call('modelgraph_svg_preview', { svg: rectangle })
    expect(recovery.isError, text(recovery)).not.toBe(true)
  })

  it('rejects malformed or active SVG rather than returning a partial successful preview', async () => {
    const { call } = await connect()
    for (const svg of ['<svg><path></svg>', '<svg><script>alert(1)</script><rect width="10" height="10"/></svg>', '<svg><rect width="10" height="10" onload="alert(1)"/></svg>']) {
      const result = await call('modelgraph_svg_preview', { svg })
      expect(result.isError).toBe(true)
      expect(text(result)).toMatch(/malformed|static|forbidden/i)
      expect(result.content.every(item => item.type === 'text')).toBe(true)
    }
  })

  it('rejects invalid base64 and oversized decoded fonts before SVG interpretation', async () => {
    const { call } = await connect()
    for (const font of ['not%base64', 'a===']) {
      const result = await call('modelgraph_svg_preview', { svg: rectangle, fonts: [font] })
      expect(result.isError).toBe(true)
      expect(text(result)).toMatch(/invalid base64/i)
    }
    // This still fits the base64 schema length but decodes to one byte over the font limit.
    const oversized = Buffer.alloc(4 * 1024 * 1024 + 1).toString('base64')
    const result = await call('modelgraph_svg_preview', { svg: rectangle, fonts: [oversized] })
    expect(result.isError).toBe(true)
    expect(text(result)).toMatch(/font.*4 MiB/i)
  })

  it('checks UTF-8 bytes when non-ASCII SVG passes the schema character limit', async () => {
    const { call } = await connect()
    const svg = `<svg xmlns="http://www.w3.org/2000/svg"><!--${'я'.repeat(2 * 1024 * 1024)}--></svg>`
    expect(svg.length).toBeLessThan(4 * 1024 * 1024)
    expect(Buffer.byteLength(svg, 'utf8')).toBeGreaterThan(4 * 1024 * 1024)
    const result = await call('modelgraph_svg_preview', { svg })
    expect(result.isError).toBe(true)
    expect(text(result)).toMatch(/SVG.*4 MiB/)
    expect(result.content.every(item => item.type === 'text')).toBe(true)
  })

  it('exports an actual ModelGraph projection as SVG and reimports the same area', async () => {
    const { call } = await connect()
    const document = { language: 'modelgraph/1', units: 'mm', parameters: [], nodes: [{ id: 'part', op: 'box', size: [20, 10, 4] }], root: 'part' }
    const exported = await call('modelgraph_svg_export', { document, axis: 'x' })
    expect(exported.isError).not.toBe(true)
    const output = resource(exported)
    expect(output.mimeType).toBe('image/svg+xml')
    expect(output.uri).toMatch(/^modelgraph:\/\/export\/[a-f0-9]{64}\.svg$/)
    const model = extrusion(await call('modelgraph_svg_extrude', { svg: output.text, height: 2 }))
    expect(model.analysis.volume).toBeCloseTo(80, 5)
  })
})
