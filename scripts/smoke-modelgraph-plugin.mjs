import { spawn } from 'node:child_process'
import { createInterface } from 'node:readline'
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
const folder = resolve(process.argv[2] ?? '.local-integrations')
const config = JSON.parse(readFileSync(resolve(folder, 'claude-desktop.json'), 'utf8')).mcpServers.modelgraph
const child = spawn(config.command, config.args, { cwd: '/tmp', env: { ...process.env, ...config.env }, stdio: ['pipe', 'pipe', 'pipe'], shell: false })
let next = 0
const pending = new Map()
let stderr = ''
child.stderr.on('data', chunk => { stderr = (stderr + chunk).slice(-4000) })
const abort = error => { for (const item of pending.values()) item.reject(error); pending.clear() }
child.on('error', abort)
child.on('exit', code => abort(new Error(`Server exited: ${code}; ${stderr}`)))
const lines = createInterface({ input: child.stdout })
lines.on('line', line => {
  try {
    const value = JSON.parse(line)
    const item = pending.get(value.id)
    if (item) { pending.delete(value.id); value.error ? item.reject(new Error(JSON.stringify(value.error))) : item.resolve(value.result) }
  } catch (error) { abort(error) }
})
const timeout = setTimeout(() => { abort(new Error('MCP smoke timed out')); child.kill() }, 30000)
const call = (method, params) => new Promise((resolve, reject) => {
  const id = ++next
  pending.set(id, { resolve, reject })
  child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n')
})
try {
  await call('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'modelgraph-plugin-smoke', version: '1' } })
  child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n')
  const ownLanguage = await call('tools/call', { name: 'modelgraph_nurbs_language', arguments: {} })
  const ownDocument = ownLanguage.structuredContent.surface_example
  const ownBuild = await call('tools/call', { name: 'modelgraph_nurbs_build', arguments: { document: ownDocument } })
  const ownImages = ownBuild.content.filter(item => item.type === 'image')
  if (ownBuild.isError || ownImages.length !== 3 || ownBuild.structuredContent.execution_target !== 'own-nurbs') throw new Error('Own NURBS build failed: ' + JSON.stringify(ownBuild.structuredContent))
  if (process.env.MODELGRAPH_PREVIEW_PATH) writeFileSync(process.env.MODELGRAPH_PREVIEW_PATH, Buffer.from(ownImages[2].data, 'base64'))
  const ownExport = await call('tools/call', { name: 'modelgraph_nurbs_export', arguments: { document: ownDocument, format: 'stl' } })
  if (ownExport.isError || !Buffer.from(ownExport.content.find(item => item.type === 'resource').resource.blob, 'base64').toString().includes('facet normal')) throw new Error('Own NURBS STL failed')
  console.log('PASS: own NURBS kernel, three PNG images and closed STL through generated stdio plugin.')
  const resource = await call('resources/read', { uri: 'openscad://language/modelgraph-1' })
  const contract = JSON.parse(resource.contents[0].text)
  const loft = await call('tools/call', { name: 'modelgraph_check', arguments: { document: contract.loft_example } })
  if (loft.isError || Math.abs(loft.structuredContent.analysis.volume - 14000/3) > 0.001) throw new Error('Loft validation failed')
  const checked = await call('tools/call', { name: 'modelgraph_check', arguments: { document: contract.units_example } })
  if (checked.isError || Math.abs(checked.structuredContent.analysis.volume - 1600) > 1e-6) throw new Error('Geometry validation failed')
  const report = await call('tools/call', { name: 'modelgraph_report', arguments: { document: contract.units_example } })
  if (report.isError || report.content.filter(item => item.type === 'image').length !== 3) throw new Error('Image report validation failed')
  const solved = await call('tools/call', { name: 'modelgraph_check', arguments: { document: contract.sketch_example } })
  if (solved.isError || solved.structuredContent.sketch_solutions[0].status !== 'solved' || Math.abs(solved.structuredContent.analysis.volume - 600) > 0.001) throw new Error('Sketch solve validation failed')
  const assembly = { ...contract.assembly_example, parameters: [{ id: 'gap', value: -1 }] }
  const interference = await call('tools/call', { name: 'modelgraph_interference', arguments: { document: assembly } })
  if (interference.isError || interference.structuredContent.status !== 'overlap' || Math.abs(interference.structuredContent.pairs[0].intersection_volume_mm3 - 200) > 0.001) throw new Error('Assembly interference validation failed')
  const built = checked.structuredContent
  const rejected = await call('tools/call', { name: 'modelgraph_set_parameters', arguments: { document: built.document, expected_document_sha256: built.document_sha256, updates: [{ id: 'wall', value: 0.6 }] } })
  if (!rejected.isError || rejected.structuredContent.error.code !== 'constraint_failed') throw new Error('Constraint validation failed')
  console.log('PASS: generated launcher, MCP handshake, language resource, real geometry (1600 mm3), three PNG views, solved sketch (600 mm3), structured constraint error, actual assembly overlap (200 mm3), closed loft (4666.67 mm3).')
} finally {
  clearTimeout(timeout)
  lines.close()
  child.stdin.end()
  child.kill('SIGTERM')
}
