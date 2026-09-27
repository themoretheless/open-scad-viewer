import assert from 'node:assert/strict'
import {readFile, readdir, mkdir, writeFile, mkdtemp, rm} from 'node:fs/promises'
import {tmpdir} from 'node:os'
import {join, resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {build, preview} from 'vite'
import {chromium} from 'playwright'

const output = resolve(process.env.BUILD_RECOVERY_OUTPUT || 'tmp/qualification/build-recovery')
const port = Number(process.env.BUILD_RECOVERY_PORT || 4185)
const scratch = await mkdtemp(join(tmpdir(), 'osv-build-recovery-'))
let server, browser, page
let phase = 'compile harness'
const pageErrors = []
try {
  await build({configFile: false, publicDir: false, build: {outDir: scratch,
    lib: {entry: resolve('tools/browser-qualification/build-recovery.ts'), formats: ['es'], fileName: () => 'harness.js'},
  }})
  const workers = (await readdir('dist/assets')).filter(file => /^geometry\.worker-.*\.js$/.test(file))
  assert.equal(workers.length, 1, 'Build the production app before running qualification')
  server = await preview({preview: {host: '127.0.0.1', port, strictPort: true}})
  browser = await chromium.launch({headless: true, ...(process.env.CHROMIUM_EXECUTABLE ? {executablePath: process.env.CHROMIUM_EXECUTABLE} : {})})
  page = await browser.newPage()
  page.on('pageerror', error => pageErrors.push(error.message))
  await page.route('**/__build-recovery.js', route => route.fulfill({contentType: 'text/javascript', path: join(scratch, 'harness.js')}))
  await page.route('**/__build-recovery', route => route.fulfill({contentType: 'text/html', body: '<!doctype html><title>Geometry build recovery qualification</title>'}))
  await page.goto(`http://127.0.0.1:${port}/__build-recovery`)
  phase = 'coordinator and production-worker scenarios'
  const result = await page.evaluate(async worker => (await import('/__build-recovery.js')).run(`/assets/${worker}`), workers[0])
  assert.deepEqual(pageErrors, [])
  assert.equal(result.reports.length, 6)
  assert.ok(result.reports.every(report => report.passed))
  await mkdir(output, {recursive: true})
  await writeFile(join(output, 'coordinator-report.json'), JSON.stringify({timestamp: new Date().toISOString(), browser: browser.version(), ...result}, null, 2) + '\n')
  phase = 'App cancel and rebuild controls'
  // This uses the actual App controls and production 30s watchdog unchanged.
  await page.addInitScript(() => {
    localStorage.setItem('scad-lang', 'en')
    localStorage.setItem('scad-auto', 'false')
    const NativeWorker = window.Worker
    const instrumentation = window.__buildRecovery = {requests: [], events: [], terminations: 0}
    window.Worker = class extends NativeWorker {
      constructor(...args) {
        super(...args)
        this.addEventListener('message', event => {
          const data = event.data
          if (!data?.protocolVersion || !data.status) return
          instrumentation.events.push({status: data.status, phase: data.phase, jobId: data.jobId, revision: data.documentRevision, volume: data.volume, error: data.error})
        })
      }
      postMessage(message, ...args) {
        if (message?.type === 'build') instrumentation.requests.push({jobId: message.jobId, revision: message.documentRevision, source: message.source})
        return super.postMessage(message, ...args)
      }
      terminate() {
        instrumentation.terminations++
        return super.terminate()
      }
    }
  })
  await page.goto(`http://127.0.0.1:${port}/`)
  await page.getByRole('button', {name: 'Source', exact: true}).click()
  const waitForSourceSuccess = async (source, volume) => {
    await page.waitForFunction(({source, volume}) => {
      const observed = window.__buildRecovery
      const request = observed.requests.findLast(request => request.source === source)
      return request && observed.events.some(event => event.jobId === request.jobId && event.status === 'succeeded' && Math.abs(event.volume - volume) < 1e-6)
    }, {source, volume}, {timeout: 30_000})
  }
  const render = page.getByRole('button', {name: 'Render', exact: true})
  await render.waitFor({timeout: 30_000})
  await page.waitForFunction(() => window.__buildRecovery.events.some(event => event.status === 'succeeded'), undefined, {timeout: 30_000})
  await page.locator('textarea.code-input').fill('cube(2);')
  await render.click()
  await waitForSourceSuccess('cube(2);', 8)
  await page.waitForFunction(() => document.querySelector('footer.stats strong')?.textContent === '1')
  const heavySource = 'union() { for (i=[0:255]) translate([i*3,0,0]) sphere(r=1,$fn=48); }'
  await page.locator('textarea.code-input').fill(heavySource)
  await render.click()
  await page.waitForFunction(source => {
    const observed = window.__buildRecovery
    const request = observed.requests.findLast(request => request.source === source)
    return request && observed.events.some(event => event.jobId === request.jobId && event.status === 'progress' && event.phase === 'compiling')
  }, heavySource, {timeout: 30_000})
  await page.getByRole('button', {name: 'Cancel build', exact: true}).click()
  await page.waitForFunction(() => [...document.querySelectorAll('button')].some(button => button.textContent.trim() === 'Render' && !button.disabled))
  assert.equal(await render.isEnabled(), true, 'Cancel should re-enable Render')
  assert.equal(await page.locator('footer.stats strong').first().textContent(), '1', 'Cancel should retain the previous scene')
  assert.equal(await page.getByRole('button', {name: 'Cancel build', exact: true}).count(), 0)
  const cancelledJob = await page.evaluate(source => window.__buildRecovery.requests.findLast(request => request.source === source).jobId, heavySource)
  await page.locator('textarea.code-input').fill('cube(3);')
  await render.click()
  await waitForSourceSuccess('cube(3);', 27)
  await page.waitForFunction(() => document.querySelector('footer.stats strong')?.textContent === '1')
  const appSmoke = await page.evaluate(cancelledJob => {
    const observed = window.__buildRecovery
    return {
      initialCubeVolume: 8,
      recoveredCubeVolume: 27,
      uiMeshCount: document.querySelector('footer.stats strong')?.textContent,
      cancelledJob,
      cancelledJobPublished: observed.events.some(event => event.jobId === cancelledJob && event.status === 'succeeded'),
      terminations: observed.terminations,
      assertions: ['Cancel build button terminates active work', 'last scene retained after cancel', 'Render re-enabled', 'next real cube succeeds with exact volume'],
      events: observed.events,
    }
  }, cancelledJob)
  assert.equal(appSmoke.cancelledJobPublished, false, 'Cancelled App build returned a success')
  assert.ok(appSmoke.terminations >= 1, 'Cancel should terminate the active worker')
  assert.deepEqual(pageErrors, [])
  await page.screenshot({path: join(output, 'app-recovered.png')})
  const hashes = {}
  for (const file of [`dist/assets/${workers[0]}`, 'public/wasm/geometry-kernel.wasm', 'src/services/buildCoordinator.ts', 'src/workers/geometry.worker.ts', 'src/App.vue', 'tools/browser-qualification/build-recovery.ts', 'tools/browser-qualification/build-recovery.mjs']) {
    hashes[file] = createHash('sha256').update(await readFile(file)).digest('hex')
  }
  await mkdir(output, {recursive: true})
  await writeFile(join(output, 'report.json'), JSON.stringify({timestamp: new Date().toISOString(), browser: browser.version(), hashes, ...result, appSmoke, pageErrors}, null, 2) + '\n')
  console.log(JSON.stringify({output, scenarios: result.reports.map(({name, elapsedMs, diagnostics}) => ({name, elapsedMs, diagnostics})), appSmoke: {...appSmoke, events: undefined}}, null, 2))
} catch (error) {
  await mkdir(output, {recursive: true})
  const instrumentation = page ? await page.evaluate(() => window.__buildRecovery ?? null).catch(() => null) : null
  await writeFile(join(output, 'failure.json'), JSON.stringify({timestamp: new Date().toISOString(), browser: browser?.version(), phase, error: String(error), stack: error?.stack, pageErrors, instrumentation}, null, 2) + '\n')
  if (page) await page.screenshot({path: join(output, 'failure.png')}).catch(() => {})
  throw error
} finally {
  try { await browser?.close() } finally {
    try { await server?.close() } finally { await rm(scratch, {recursive: true, force: true}) }
  }
}
