// Runs Rust/WASM protocol checks in Chromium; makes no UI interaction claim.
import {createServer} from 'vite'
import {readFileSync, writeFileSync, mkdtempSync, rmSync} from 'node:fs'
import {join} from 'node:path'
import {tmpdir} from 'node:os'
import {createHash} from 'node:crypto'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
const bytes = readFileSync(process.argv[2] ?? 'crates/geometry-bridge/tests/fixtures/general-frame-cap-certificate-v1.json')
const output = process.argv[3] ?? '/tmp/osv-general-boundary-browser.json'
const cacheDir = mkdtempSync(join(tmpdir(), 'osv-boundary-vite-'))
const server = await createServer({configFile: false, root: process.cwd(), appType: 'custom', logLevel: 'warn', cacheDir,
  optimizeDeps: {noDiscovery: true},
  server: {host: '127.0.0.1', port: 0}, plugins: [{name: 'general-boundary-probe', configureServer(server) {
    server.middlewares.use((req, res, next) => {
      if (req.url === '/general-boundary-fixture') {res.setHeader('Content-Type', 'application/json'); res.end(bytes); return}
      if (req.url === '/general-boundary-probe') {res.setHeader('Content-Type', 'text/html'); res.end('<!doctype html><title>Rust boundary certificate</title>'); return}
      next()
    })
  }}]})
let browser
try {
  await server.listen()
  const {playwright} = await loadQualificationPlaywrightPackage()
  browser = await playwright.chromium.launch({headless: true, channel: 'chromium'})
  const page = await browser.newPage(), errors = []
  page.on('pageerror', e => errors.push(e.message))
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/general-boundary-probe`)
  await page.evaluate(() => {void (async () => {try {
    const {callGeometryRust, warmGeometryKernel} = await import('/src/services/geometry/kernel.ts')
    const {runGeneralBoundaryChecks} = await import('/scripts/generalBoundaryChecks.mjs')
    await warmGeometryKernel()
    const fixture = await (await fetch('/general-boundary-fixture')).json()
    window.boundaryReport = runGeneralBoundaryChecks(fixture, ({op, ...args}) => callGeometryRust(op, args))
  } catch (e) {window.boundaryFailure = String(e?.stack ?? e)}})()})
  await page.waitForFunction(() => window.boundaryReport || window.boundaryFailure, null, {timeout: 120000})
  const {report, failure} = await page.evaluate(() => ({report: window.boundaryReport, failure: window.boundaryFailure}))
  if (failure || errors.length) throw Error(failure ?? errors.join('\n'))
  writeFileSync(output, JSON.stringify({...report, browser: browser.version(),
    fixtureSha256: createHash('sha256').update(bytes).digest('hex'), artifact: sweepStepArtifactProvenance()}, null, 2) + '\n')
  console.log(`Browser general boundary: ${report.passed} checks passed`)
} finally {await browser?.close(); await server.close(); rmSync(cacheDir, {recursive: true, force: true})}
