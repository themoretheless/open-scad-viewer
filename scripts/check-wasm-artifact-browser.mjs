import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {createServer} from 'vite'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'

const artifact = readFileSync(new URL('../public/wasm/geometry-kernel.wasm', import.meta.url))
const identity = {byteLength: artifact.length, sha256: createHash('sha256').update(artifact).digest('hex')}
const damaged = Buffer.from(artifact)
damaged[damaged.length - 1] ^= 1
function artifactResponse(request, response, next) {
  const mode = request.url?.match(/^\/__artifact\/(valid|damaged|truncated|oversized|stalled)$/)?.[1]
  if (!mode) return next()
  response.setHeader('Content-Type', 'application/wasm')
  if (mode === 'stalled') {
    response.write(artifact.subarray(0, 8))
    return
  }
  response.end(mode === 'valid' ? artifact : mode === 'damaged' ? damaged
    : mode === 'truncated' ? artifact.subarray(0, artifact.length - 1) : Buffer.concat([artifact, Buffer.from([0])]))
}
const server = await createServer({logLevel: 'silent', server: {host: '127.0.0.1', port: 0}, plugins: [{
  name: 'wasm-artifact-probe', configureServer(server) { server.middlewares.use(artifactResponse) },
}]})
let browser
try {
  await server.listen()
  const {playwright} = await loadQualificationPlaywrightPackage()
  browser = await playwright.chromium.launch({headless: true,
    ...(process.env.CHROMIUM_EXECUTABLE ? {executablePath: process.env.CHROMIUM_EXECUTABLE} : {})})
  const page = await browser.newPage()
  // Use a minimal same-origin page, without initializing the application kernel.
  const origin = server.resolvedUrls.local[0]
  await page.route(`${origin}__probe`, route => route.fulfill({contentType: 'text/html', body: '<!doctype html><title>WASM artifact check</title>'}))
  await page.goto(`${origin}__probe`)
  const report = await page.evaluate(async identity => {
    const {compileStreamingWasm} = await import('/src/services/wasmStreaming.ts')
    const {assertVerifiedWasmModule} = await import('/src/services/wasmArtifact.ts')
    const {setOptionalWasmCompiler, compileOptionalWasm} = await import('/src/services/wasmCompilation.ts')
    const results = []
    for (const mode of ['valid', 'damaged', 'truncated', 'oversized', 'stalled']) {
      const start = performance.now()
      const module = await compileStreamingWasm(`/__artifact/${mode}`, identity)
      if (module) assertVerifiedWasmModule(module, identity)
      results.push({mode, accepted: module !== null, elapsedMs: performance.now() - start,
        hasAbi: module ? WebAssembly.Module.exports(module).some(item => item.name === 'abi_request') : false})
    }
    setOptionalWasmCompiler(async () => new WebAssembly.Module(new Uint8Array([0,97,115,109,1,0,0,0])))
    let unverifiedCode
    try { await compileOptionalWasm('/unused', identity) }
    catch (error) { unverifiedCode = error.code }
    finally { setOptionalWasmCompiler(undefined) }
    return {results, unverifiedCode}
  }, identity)
  console.log(JSON.stringify({browser: browser.version(), artifact: identity,
    scope: 'Real browser verified network compilation and refusal smoke; not a latency benchmark or qualification', ...report}, null, 2))
  assert.equal(report.results[0].accepted, true)
  assert.equal(report.results[0].hasAbi, true)
  for (const result of report.results.slice(1)) assert.equal(result.accepted, false, result.mode)
  assert.ok(report.results.at(-1).elapsedMs >= 1_900 && report.results.at(-1).elapsedMs < 5_000)
  assert.equal(report.unverifiedCode, 'WASM_ARTIFACT_MISMATCH')
} finally {
  await browser?.close()
  await server.close()
}
