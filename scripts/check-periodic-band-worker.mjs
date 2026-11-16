import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFileSync, readdirSync } from 'node:fs'
import { resolve } from 'node:path'
import { createServer } from 'vite'
import { loadQualificationPlaywrightPackage } from './qualificationPlaywrightPackage.mjs'

const candidate = process.argv.slice(2).find(arg => !arg.startsWith('--'))
const dist = process.argv.includes('--dist') ? resolve(candidate ?? 'dist') : null
const root = candidate && !dist ? resolve(candidate) : null
const embedded = process.argv.includes('--embedded')
const wasm = readFileSync(dist ? resolve(dist, 'wasm/geometry-kernel.wasm')
  : root ? resolve(root, 'public/geometry-kernel.wasm') : 'public/wasm/geometry-kernel.wasm')
const workerEntry = dist ? readdirSync(resolve(dist, 'assets')).find(name => /^mainSolid\.worker-.*\.js$/.test(name)) : null
if (dist) assert.ok(workerEntry, 'Expected the production CAD Worker in dist/assets')
const alias = root ? [
  { find: '../../generated/geometry-kernels/identity', replacement: resolve(root, 'embedded/identity.ts') },
  { find: '../../generated/geometry-kernels/bytes', replacement: resolve(root, 'embedded/bytes.ts') },
] : []
const server = await createServer({
  configFile: false,
  ...(dist ? { publicDir: dist } : {}),
  logLevel: 'silent',
  resolve: { alias },
  server: { host: '127.0.0.1', port: 0, fs: { allow: [process.cwd(), ...(root ? [root] : []), ...(dist ? [dist] : [])] } },
  plugins: [{
    name: 'periodic-band-probe',
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        if (request.url === '/__periodic-band') {
          response.setHeader('Content-Type', 'text/html')
          response.end('<!doctype html><title>Periodic band Worker check</title>')
        } else if (new URL(request.url, 'http://localhost').pathname === '/wasm/geometry-kernel.wasm') {
          if (embedded) {
            response.writeHead(404).end()
          } else {
            response.setHeader('Content-Type', 'application/wasm')
            response.end(wasm)
          }
        } else next()
      })
    },
  }],
})

const profile = [[1,0],[1,1],[0,1],[-1,1],[-1,0],[-1,-1],[0,-1],[1,-1],[1,0],[1,1]]
const side = {
  degreeU: 2, degreeV: 1,
  knotsU: [-1,0,0,1,1,2,2,3,3,4,4,5,5], knotsV: [0,0,1,1],
  controlPoints: profile.map(([x,y]) => [[x,y,0],[0,0,2]]),
  weights: profile.map((_, index) => Array(2).fill(index % 2 ? Math.SQRT1_2 : 1)),
  periodicU: true, periodicV: false,
}
const plane = z => ({
  degreeU: 1, degreeV: 1, knotsU: [0,0,1,1], knotsV: [0,0,1,1],
  controlPoints: [[[-2,-2,z],[-2,2,z]],[[2,-2,z],[2,2,z]]],
  weights: [[1,1],[1,1]], periodicU: false, periodicV: false,
})
const input = { side, planes: [plane(1.5), plane(1)], toleranceMm: 1e-7 }

let browser
try {
  await server.listen()
  const { playwright } = await loadQualificationPlaywrightPackage()
  browser = await playwright.chromium.launch({
    headless: true,
    ...(process.env.CHROMIUM_EXECUTABLE ? { executablePath: process.env.CHROMIUM_EXECUTABLE } : {}),
  })
  const page = await browser.newPage()
  let workers = 0
  let fetched = false
  page.on('worker', () => { workers++ })
  page.on('response', response => {
    if (new URL(response.url()).pathname === '/wasm/geometry-kernel.wasm' && response.ok()) fetched = true
  })
  await page.goto(server.resolvedUrls.local[0] + '__periodic-band')
  const { report, refused, elapsedMs } = await page.evaluate(async ({ input, workerEntry }) => {
    const started = performance.now()
    let compute
    let close
    if (workerEntry) {
      const worker = new Worker('/assets/' + workerEntry, { type: 'module' })
      let nextId = 1
      compute = request => new Promise((resolve, reject) => {
        const id = nextId++
        const timer = setTimeout(() => { cleanup(); reject(new Error('CAD Worker timeout')) }, 20000)
        const accept = ({ data }) => {
          if (data.id !== id || data.kind !== 'periodicBand' || typeof data.ok !== 'boolean') return
          cleanup()
          if (data.ok) resolve(data.result)
          else reject(new Error(data.error.message))
        }
        const fail = error => { cleanup(); reject(new Error(error.message)) }
        function cleanup() {
          clearTimeout(timer)
          worker.removeEventListener('message', accept)
          worker.removeEventListener('error', fail)
        }
        worker.addEventListener('message', accept)
        worker.addEventListener('error', fail)
        worker.postMessage({ version: 1, id, job: { kind: 'periodicBand', request } })
      })
      close = () => worker.terminate()
    } else {
      const { computePeriodicBand, cancelMainSolid } = await import('/src/kernel/mainSolidWorker.ts')
      compute = request => computePeriodicBand(request, { timeoutMs: 20000 })
      close = cancelMainSolid
    }
    try {
      const report = await compute(input)
      const elapsedMs = performance.now() - started
      const refused = await compute(
        { ...input, planes: [input.planes[0], input.planes[0]] },
      ).then(() => false, error => /distinct/.test(error.message))
      return { report, refused, elapsedMs }
    } finally {
      close()
    }
  }, { input, workerEntry })
  assert.equal(workers, 1)
  assert.equal(fetched, !embedded)
  assert.equal(refused, true)
  assert.deepEqual(report.source, { op: 'brep_qualify_periodic_band', ...input })
  assert.equal(report.bodies, 1)
  assert.equal(report.faces, 3)
  assert.equal(report.edges, 8)
  assert.equal(report.boundaryEdges, 0)
  const [lo, hi] = report.volumeBoundsMm3
  const expected = 7 * Math.PI / 96
  assert.ok(lo > 0 && lo <= expected && expected <= hi && hi - lo < 0.001)
  console.log(JSON.stringify({
    browser: browser.version(), workers, fetched, embedded, production: Boolean(dist), refused, elapsedMs,
    sha256: createHash('sha256').update(wasm).digest('hex'), bytes: wasm.length,
    volumeBoundsMm3: [lo, hi],
  }))
} finally {
  await browser?.close()
  await server.close()
}
