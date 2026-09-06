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
  it('discovers, builds and exports without persistent session state', async () => {
    const { call } = await start()
    const init = await (await call('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'test', version: '1' } })).json()
    expect(init.result.serverInfo.name).toBe('modelgraph')
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
