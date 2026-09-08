import { readFileSync } from 'node:fs'
import { afterEach, describe, expect, it } from 'vitest'
import { createModelGraphHttpServer } from '../src/mcp/modelGraphHttp'
import { MODELGRAPH_UNITS_EXAMPLE } from '../src/services/modelGraph'
const servers: ReturnType<typeof createModelGraphHttpServer>[] = []
afterEach(async () => { for (const server of servers.splice(0)) { server.closeAllConnections(); await new Promise<void>(resolve => server.close(() => resolve())) } })
async function start() {
  const server = createModelGraphHttpServer(); servers.push(server)
  await new Promise<void>((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve) })
  const address = server.address() as { port: number }
  const url = `http://127.0.0.1:${address.port}/mcp`
  const call = (method: string, params: unknown) => fetch(url, { method: 'POST', headers: { 'Content-Type': 'application/json', Accept: 'application/json, text/event-stream', 'MCP-Protocol-Version': '2025-11-25' }, body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }) })
  return { url, call }
}
describe('remote ModelGraph MCP', () => {
  it('builds generic block functions and named records through MCP', async () => {
    const {call}=await start()
    const tool=async(name:string,args:unknown)=>(await (await call('tools/call',{name,arguments:args})).json()).result
    const compiled=await tool('modelgraph_text_compile',{source:readFileSync('examples/modelgraph-text/generic-functions.scad','utf8')})
    expect(compiled.isError,JSON.stringify(compiled)).not.toBe(true)
    const checked=await tool('modelgraph_check',{document:compiled.structuredContent.document})
    expect(checked.isError,JSON.stringify(checked)).not.toBe(true)
    expect(checked.structuredContent.analysis.volume).toBeCloseTo(2400)
  })
  it('compiles own NURBS text and exports through the indicated MCP route', async () => {
    const {call}=await start()
    const tool=async(name:string,args:unknown)=>(await (await call('tools/call',{name,arguments:args})).json()).result
    const compiled=await tool('modelgraph_text_compile',{source:readFileSync('examples/modelgraph-text/nurbs-boolean.scad','utf8')})
    expect(compiled.isError).not.toBe(true)
    expect(compiled.structuredContent.execution_target).toBe('own-nurbs')
    const exported=await tool('modelgraph_nurbs_export',{document:compiled.structuredContent.document,format:'stl'})
    expect(exported.isError,JSON.stringify(exported)).not.toBe(true)
    expect(exported.structuredContent.report.mesh.signedVolumeMm3).toBeCloseTo(1200,7)
  })

  it('discovers, builds and exports without persistent session state', async () => {
    const { call } = await start()
    const init = await (await call('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'test', version: '1' } })).json()
    expect(init.result.serverInfo.name).toBe('modelgraph')
    expect(init.result.instructions).toContain('First call modelgraph_language')
    const language = await (await call('tools/call', { name: 'modelgraph_language', arguments: {} })).json()
    const resource = await (await call('resources/read', { uri: 'openscad://language/modelgraph-1' })).json()
    expect(language.result.structuredContent).toEqual(JSON.parse(resource.result.contents[0].text))
    expect(language.result.structuredContent.mechanical_examples).toBeDefined()
    const compact = await (await call('tools/call', { name: 'modelgraph_text_compile', arguments: { source: '// @modelgraph-text/1\nparam r = 2mm range 1mm..5mm\nbody = circle(r) |> extrude(3mm)' } })).json()
    expect(compact.result.isError).toBe(false)
    const compactBuild = await (await call('tools/call', { name: 'modelgraph_check', arguments: { document: compact.result.structuredContent.document } })).json()
    expect(compactBuild.result.isError).toBe(false)
    expect(compactBuild.result.structuredContent.analysis.volume).toBeGreaterThan(30)
    expect(language.result.structuredContent.instructions).toContain('32 leaf')
    const listed = await (await call('tools/list', {})).json()
    expect(listed.result.tools.map((item: { name: string }) => item.name)).toContain('modelgraph_export')
    const built = await (await call('tools/call', { name: 'modelgraph_check', arguments: { document: MODELGRAPH_UNITS_EXAMPLE } })).json()
    expect(built.result.isError).toBe(false)
    expect(built.result.structuredContent.analysis.volume).toBeCloseTo(1600)
    const exported = await (await call('tools/call', { name: 'modelgraph_export', arguments: { document: MODELGRAPH_UNITS_EXAMPLE, format: 'stl' } })).json()
    expect(exported.result.isError).not.toBe(true)
    expect(Buffer.from(exported.result.content[0].resource.blob, 'base64').length).toBeGreaterThan(84)
    const package3mf = await (await call('tools/call', { name: 'modelgraph_export', arguments: { document: MODELGRAPH_UNITS_EXAMPLE, format: '3mf' } })).json()
    expect(package3mf.result.isError).not.toBe(true)
    expect(Buffer.from(package3mf.result.content[0].resource.blob, 'base64').readUInt32LE(0)).toBe(0x04034b50)
  },15000)
  it('preserves fluent checks through MCP compilation, reporting and export', async () => {
    const {call}=await start()
    const tool=async(name:string,args:unknown)=>(await (await call('tools/call',{name,arguments:args})).json()).result
    const source='// @modelgraph-text/1\nbody = box([2mm,3mm,4mm])\nshow body\nassert body |> hasBodies(2)'
    const compiled=await tool('modelgraph_text_compile',{source})
    expect(compiled.isError).toBe(false)
    const document=compiled.structuredContent.document
    const checked=await tool('modelgraph_check',{document})
    expect(checked.isError).toBe(true)
    expect(checked.structuredContent.checks[0]).toMatchObject({status:'failed',expected:2,actual:1})
    const report=await tool('modelgraph_report',{document})
    expect(report.isError).toBe(true)
    expect(report.structuredContent.checks[0].status).toBe('failed')
    const exported=await tool('modelgraph_export',{document,format:'3mf'})
    expect(exported.isError).toBe(true)
    expect(exported.content.some((c:{type:string})=>c.type==='resource')).toBe(false)
    const invalid=await tool('modelgraph_text_compile',{source:source+'\nvalidate 2mm |> atMost(1mm) |> message("Gap too large")'})
    expect(invalid.structuredContent.error.code).toBe('constraint_failed')
    expect(invalid.structuredContent.error.details[0]).toMatchObject({status:'failed',message:'Gap too large'})
    document.geometry_assertions[0].expected=1
    const passed=await tool('modelgraph_check',{document})
    expect(passed.isError).toBe(false)
    expect(passed.structuredContent.checks[0].status).toBe('passed')
  },15000)
  it('recomputes endpoint ranges and exports generated parts through MCP', async () => {
    const {call}=await start()
    const tool=async(name:string,args:unknown)=>(await (await call('tools/call',{name,arguments:args})).json()).result
    const source='// @modelgraph-text/1\nparam count=18 range 1..24\npart=box([1mm,1mm,1mm])\nparts=[for a in 0deg..<360deg count count => part.translate(x: 20mm).rotate(z: a)]\nshow parts'
    const compiled=await tool('modelgraph_text_compile',{source})
    expect(compiled.isError).toBe(false)
    const c=compiled.structuredContent
    expect(c.document.nodes.some((n:{op:string})=>n.op==='collect')).toBe(true)
    const checked=await tool('modelgraph_check',{document:c.document})
    expect(checked.isError).toBe(false)
    expect(checked.structuredContent.analysis.meshCount).toBe(18)
    const updated=await tool('modelgraph_set_parameters',{document:c.document,expected_document_sha256:c.document_sha256,updates:[{id:'count',value:6}]})
    expect(updated.isError).toBe(false)
    const rebuilt=await tool('modelgraph_check',{document:updated.structuredContent.document})
    expect(rebuilt.structuredContent.analysis.meshCount).toBe(6)
    const exported=await tool('modelgraph_export',{document:updated.structuredContent.document,format:'3mf'})
    expect(exported.isError).not.toBe(true)
    expect(exported.content[0].resource.blob.length).toBeGreaterThan(100)
    const language=await tool('modelgraph_language',{})
    expect(language.structuredContent.text_guide).toContain('0deg..<360deg count 18')
  },15000)
  it('generates an internal thread through HTTP MCP',async()=>{
    const {call}=await start()
    const generated = await (await call('tools/call',{name:'modelgraph_generate',arguments:{kind:'thread',length:6,internal:true}})).json()
    expect(generated.result.isError).not.toBe(true)
    expect(generated.result.structuredContent.analysis.volume).toBeGreaterThan(0)
    expect(generated.result.structuredContent.mechanical_reports[0].internal).toBe(true)
  },15000)
  it('rejects browser origins, unsupported methods and oversized requests', async () => {
    const { url } = await start()
    expect((await fetch(url)).status).toBe(405)
    expect((await fetch(url, { method: 'POST', headers: { Origin: 'https://example.org' } })).status).toBe(403)
    expect((await fetch(url, { method: 'POST', body: 'x'.repeat(262145) })).status).toBe(413)
  })
})
