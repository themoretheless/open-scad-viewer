import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createServer } from 'vite'
import { loadQualificationPlaywrightPackage } from './qualificationPlaywrightPackage.mjs'
const [fixturePath, wasmPath, comparisonPath] = process.argv.slice(2)
assert(fixturePath && wasmPath, 'Expected a closed SS seam request and a geometry WASM artifact')
const fixture = JSON.parse(readFileSync(fixturePath, 'utf8'))
const wasm = readFileSync(wasmPath)
const comparison = comparisonPath ? readFileSync(comparisonPath) : null
const server = await createServer({ logLevel: 'silent', server: { host: '127.0.0.1', port: 0 } })
let browser
try {
  await server.listen()
  const { playwright } = await loadQualificationPlaywrightPackage()
  browser = await playwright.chromium.launch({
    headless: true,
    ...(process.env.CHROMIUM_EXECUTABLE ? { executablePath: process.env.CHROMIUM_EXECUTABLE } : {}),
  })
  const page = await browser.newPage()
  const origin = server.resolvedUrls.local[0]
  await page.route(origin + '__seam', route => route.fulfill({ contentType: 'text/html', body: '<!doctype html>' }))
  await page.route(origin + '__seam.wasm', route => route.fulfill({ contentType: 'application/wasm', body: wasm }))
  if (comparison)
    await page.route(origin + '__seam-comparison.wasm', route =>
      route.fulfill({ contentType: 'application/wasm', body: comparison }),
    )
  await page.goto(origin + '__seam')
  const workerSource = `self.onmessage = async ({data}) => {
    try { self.postMessage(await (${run.toString()})(data)) }
    catch (error) { self.postMessage({error:String(error)}) }
  }`
  await page.route(origin + '__seam-worker.js', route =>
    route.fulfill({
      contentType: 'text/javascript',
      body: workerSource,
    }),
  )
  const result = await page.evaluate(
    fixture =>
      new Promise((resolve, reject) => {
        const worker = new Worker('/__seam-worker.js', { type: 'module' })
        worker.onmessage = ({ data }) => {
          worker.terminate()
          resolve(data)
        }
        worker.onerror = error => {
          worker.terminate()
          reject(error.message)
        }
        worker.postMessage(fixture)
      }),
    { fixture, compare: Boolean(comparison) },
  )
  assert.equal(result.error, undefined)
  const { proof, comparisonProof, timingsMs } = result
  if (comparison) assert.deepEqual(comparisonProof, proof)
  console.log(
    JSON.stringify({
      browser: browser.version(),
      bytes: wasm.length,
      ...(comparison ? { comparisonBytes: comparison.length } : {}),
      ...proof,
      timingsMs,
    }),
  )
  assert.equal(proof.error, undefined)
  assert.equal(proof.branches, 1)
  assert.deepEqual(proof.faces, [2, 2])
  assert.ok(proof.world < fixture.toleranceMm && proof.uv.every(error => error < fixture.toleranceUv))
  assert.ok(proof.seam[0] < 0 && proof.seam[1] > 0)
} finally {
  await browser?.close()
  await server.close()
}

async function run({ fixture, compare }) {
  const { encodeBinary, decodeBinary } = await import('/src/core/valueBinaryCodec.ts')
  const paths = compare ? ['/__seam.wasm', '/__seam-comparison.wasm'] : ['/__seam.wasm']
  const kernels = []
  for (const path of paths) {
    const { instance } = await WebAssembly.instantiateStreaming(fetch(path), {})
    kernels.push(instance.exports)
  }
  function execute(w) {
    const bytes = encodeBinary(fixture)
    const p = w.abi_alloc(bytes.length)
    new Uint8Array(w.memory.buffer, p, bytes.length).set(bytes)
    const started = performance.now()
    const packed = w.abi_request(0, p, bytes.length)
    const elapsed = performance.now() - started
    const q = Number(packed & 0xffffffffn),
      n = Number(packed >> 32n)
    const reply = decodeBinary(new Uint8Array(w.memory.buffer, q, n))
    w.abi_free(q, n)
    w.abi_free(p, bytes.length)
    if (!reply.ok) throw new Error(JSON.stringify(reply.error))
    const r = reply.value
    const v = r.a.loops[r.a.faces[0].holes[0]].coedges.flatMap(edge => edge.pcurve.controlPoints.map(point => point[1]))
    return {
      elapsed,
      proof: {
        branches: r.branches,
        faces: [r.a.faces.length, r.b.faces.length],
        segments: r.segments,
        world: r.worldErrorUpper,
        uv: r.uvErrorUpper,
        seam: [Math.min(...v), Math.max(...v)],
      },
    }
  }
  if (compare) kernels.forEach(execute)
  const timingsMs = kernels.map(() => [])
  const proofs = []
  for (let round = 0; round < (compare ? 7 : 1); round++) {
    const indices = kernels.map((_, index) => index)
    if (round % 2) indices.reverse()
    for (const index of indices) {
      const { elapsed, proof } = execute(kernels[index])
      timingsMs[index].push(elapsed)
      proofs[index] = proof
    }
  }
  return { proof: proofs[0], comparisonProof: proofs[1], timingsMs }
}
