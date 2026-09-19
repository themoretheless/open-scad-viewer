import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { createServer } from 'node:http'
import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { loadQualificationPlaywrightPackage } from '../scripts/qualificationPlaywrightPackage.mjs'

const root = path.resolve('dist')
const out = path.resolve(process.env.EXACT_SOLID_BENCH_OUT || 'tmp/performance/exact-solid-browser')
const forceWasmFallback = process.env.EXACT_SOLID_WASM_FALLBACK === '1'
await mkdir(out, { recursive: true })
const clientFile = (await readdir(path.join(root, 'assets'))).find(file => /^exactSolidClient-.*\.js$/.test(file))
assert.ok(clientFile, 'Build the app before running this browser check')
const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url, 'http://localhost')
    if (forceWasmFallback && url.pathname.startsWith('/wasm/')) {
      response.writeHead(404).end()
      return
    }
    if (url.pathname === '/__bench') {
      response.setHeader('Content-Type', 'text/html')
      response.end('<!doctype html><html><body>Exact solid worker check</body></html>')
      return
    }
    const file = path.resolve(root, `.${url.pathname === '/' ? '/index.html' : decodeURIComponent(url.pathname)}`)
    if (!file.startsWith(`${root}${path.sep}`)) { response.writeHead(403).end(); return }
    response.setHeader('Content-Type', file.endsWith('.js') ? 'text/javascript' : file.endsWith('.css') ? 'text/css' : file.endsWith('.html') ? 'text/html' : file.endsWith('.wasm') ? 'application/wasm' : 'application/octet-stream')
    response.end(await readFile(file))
  } catch { response.writeHead(404).end() }
})
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
const origin = `http://127.0.0.1:${server.address().port}`
let browser
try {
  const { playwright } = await loadQualificationPlaywrightPackage()
  browser = await playwright.chromium.launch({ headless: true,
    ...(process.env.CHROMIUM_EXECUTABLE ? { executablePath: process.env.CHROMIUM_EXECUTABLE } : {}),
  })
  const page = await browser.newPage()
  await page.addInitScript(() => {
    globalThis.__successfulStreamingCompiles = 0
    const compile = WebAssembly.compileStreaming.bind(WebAssembly)
    WebAssembly.compileStreaming = async (...args) => {
      const module = await compile(...args)
      globalThis.__successfulStreamingCompiles++
      return module
    }
  })
  const pageErrors = []
  page.on('pageerror', error => pageErrors.push(error.message))
  await page.goto(`${origin}/__bench`)
  const sources = [
    'cube([10,20,30], center=true);',
    'cube(2); translate([8,0,0]) cube(3);',
    await readFile('examples/modelgraph-text/planetary-spinner.mg', 'utf8'),
  ]
  const profiler = process.env.EXACT_SOLID_PROFILE === '1' ? await page.context().newCDPSession(page) : null
  if (profiler) { await profiler.send('Profiler.enable'); await profiler.send('Profiler.start') }
  const result = await page.evaluate(async ({ clientFile, sources }) => {
    const module = await import(`/assets/${clientFile}`)
    const build = Object.values(module).find(value => typeof value === 'function')
    const samples = []
    for (const source of sources) {
      let ticks = 0, maxGapMs = 0, last = performance.now()
      const timer = setInterval(() => { const now = performance.now(); maxGapMs = Math.max(maxGapMs, now - last); last = now; ticks++ }, 16)
      const start = performance.now()
      const bodies = await build(source)
      const durationMs = performance.now() - start
      maxGapMs = Math.max(maxGapMs, performance.now() - last)
      clearInterval(timer)
      const bytes = new TextEncoder().encode(JSON.stringify(bodies.map(({ id, ...body }) => body)))
      const digest = await crypto.subtle.digest('SHA-256', bytes)
      samples.push({ bodies: bodies.length, faces: bodies.map(body => body.brep.faces.length), durationMs, ticks, maxGapMs,
        geometrySha256: Array.from(new Uint8Array(digest), x => x.toString(16).padStart(2, '0')).join('') })
    }
    let refusal
    try { await build('hull(){cube(2);translate([6,0,0])cube(2);}') } catch (error) { refusal = { name: error.name, message: error.message } }
    const controller = new AbortController()
    const start = performance.now()
    const cancelled = build(sources[2], controller.signal).then(() => 'unexpected success', error => error.name)
    setTimeout(() => controller.abort(), 100)
    const cancellation = { name: await cancelled, durationMs: performance.now() - start }
    const recovered = await build('cube(1);')
    return { samples, refusal, cancellation, recoveredBodies: recovered.length }
  }, { clientFile, sources })
  if (profiler) {
    const { profile } = await profiler.send('Profiler.stop')
    await writeFile(path.join(out, 'main-thread.cpuprofile'), JSON.stringify(profile))
    await profiler.detach()
  }
  assert.deepEqual(result.samples.map(sample => sample.bodies), [1, 2, 20])
  assert.ok(result.samples.every(sample => sample.ticks > 0))
  assert.equal(result.refusal?.name, 'InexactSolidError')
  assert.equal(result.cancellation.name, 'AbortError')
  assert.equal(result.recoveredBodies, 1)
  assert.deepEqual(pageErrors, [])
  // Exercise the real App wiring too, outside the timed isolated workloads.
  await page.setViewportSize({ width: 1440, height: 1000 })
  await page.goto(origin)
  await page.locator('.source-toggle').click()
  const sourceEditor = page.locator('textarea.code-input').last()
  const buildButton = page.getByRole('button', { name: /^(To Solid|В Solid)$/ })
  await sourceEditor.fill(sources[0])
  await buildButton.click()
  await page.waitForFunction(() => document.querySelector('.source-toggle')?.getAttribute('aria-pressed') === 'false')
  await page.waitForFunction(() => document.querySelectorAll('.scene-list .dot.body').length === 1)
  await page.locator('.source-toggle').click()
  await sourceEditor.fill(sources[1])
  await buildButton.click()
  await page.waitForFunction(() => document.querySelectorAll('.scene-list .dot.body').length === 2)
  await page.waitForFunction(() => document.querySelector('.source-toggle')?.getAttribute('aria-pressed') === 'false')
  await page.screenshot({ path: path.join(out, 'desktop.png') })
  await page.locator('.source-toggle').click()
  await sourceEditor.fill(sources[2])
  await buildButton.click()
  await page.locator('.editor-toolbar').getByRole('button', { name: /^(Cancel|Отмена)$/ }).click()
  await buildButton.waitFor({ state: 'visible' })
  assert.equal(await page.locator('.source-toggle').getAttribute('aria-pressed'), 'true')
  assert.equal(await page.locator('.scene-list .dot.body').count(), 2)
  await page.setViewportSize({ width: 390, height: 844 })
  await page.screenshot({ path: path.join(out, 'mobile.png') })
  result.app = { sourceBuild: true, groupReplacement: true, cancellationPreservesSceneAndEditor: true }
  await page.setViewportSize({ width: 1440, height: 1000 })
  await page.getByRole('button', { name: /^(Generators|Генераторы)$/ }).click()
  const generator = page.locator('dialog.mechanical-dialog')
  await generator.waitFor({ state: 'visible' })
  for (const kind of ['gear', 'planetary_gears', 'thread']) {
    await generator.locator('select').selectOption(kind)
    await generator.locator('button[type="submit"]').click()
    await generator.locator('.generated').waitFor({ state: 'visible' })
    assert.equal(await generator.getByRole('alert').count(), 0)
  }
  await generator.locator('select').selectOption('gear')
  const teeth = generator.getByLabel(/^(Teeth|Число зубьев)$/)
  await teeth.fill('2')
  await generator.locator('button[type="submit"]').click()
  await generator.getByRole('alert').waitFor({ state: 'visible' })
  assert.equal(await generator.locator('.generated').count(), 0)
  await teeth.fill('24')
  await generator.locator('button[type="submit"]').click()
  await generator.locator('.generated').waitFor({ state: 'visible' })
  assert.equal(await generator.getByRole('alert').count(), 0)
  await generator.getByRole('button', { name: /^(Close|Закрыть)$/ }).click()
  assert.equal(await page.locator('.scene-list .dot.body').count(), 2)
  assert.equal(await sourceEditor.inputValue(), sources[2])
  result.app.mechanicalGenerator = { allKinds: true, invalidInputRefused: true, recovered: true }
  result.streaming = { forcedFallback: forceWasmFallback,
    successfulCompiles: await page.evaluate(() => globalThis.__successfulStreamingCompiles) }
  if (forceWasmFallback) assert.equal(result.streaming.successfulCompiles, 0)
  else assert.ok(result.streaming.successfulCompiles > 0, 'Browser startup must exercise streaming compilation')
  assert.deepEqual(pageErrors, [])
  const assets = []
  for (const file of (await readdir(path.join(root, 'assets'))).filter(file => file.endsWith('.js')).sort()) {
    const bytes = await readFile(path.join(root, 'assets', file))
    assets.push({ file, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') })
  }
  const sourceSha256 = sources.map(source => createHash('sha256').update(source).digest('hex'))
  await writeFile(path.join(out, 'report.json'), JSON.stringify({ browser: browser.version(), platform: process.platform,
    arch: process.arch, profiled: !!profiler, clientFile, sourceSha256, assets, ...result, pageErrors }, null, 2))
  console.log(JSON.stringify(result, null, 2))
} finally {
  await browser?.close()
  await new Promise(resolve => server.close(resolve))
}
