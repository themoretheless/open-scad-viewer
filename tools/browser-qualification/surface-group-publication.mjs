import assert from 'node:assert/strict'
import {readFile, readdir, mkdir, writeFile, mkdtemp} from 'node:fs/promises'
import {tmpdir} from 'node:os'
import {join, resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {build, preview} from 'vite'
import {chromium} from 'playwright'

const output = resolve(process.env.SURFACE_PUBLICATION_OUTPUT || 'tmp/performance/surface-group-publication')
const scratch = await mkdtemp(join(tmpdir(), 'osv-surface-publication-'))
await build({configFile: false, publicDir: false, build: {outDir: scratch,
  lib: {entry: resolve('tools/browser-qualification/surface-group-publication.ts'), formats: ['es'], fileName: () => 'harness.js'},
}})
const workers = (await readdir('dist/assets')).filter(f => /^geometry\.worker-.*\.js$/.test(f))
assert.equal(workers.length, 1)
let server, browser
try {
  server = await preview({preview: {host: '127.0.0.1', port: 4184, strictPort: true}})
  browser = await chromium.launch({headless: true, ...(process.env.CHROMIUM_EXECUTABLE ? {executablePath: process.env.CHROMIUM_EXECUTABLE} : {})})
  const page = await browser.newPage()
  const errors = []
  page.on('pageerror', error => errors.push(error.message))
  await page.route('**/__surface-harness.js', route => route.fulfill({contentType: 'text/javascript', path: join(scratch, 'harness.js')}))
  await page.route('**/__surface-bench', route => route.fulfill({contentType: 'text/html', body: '<!doctype html><title>Surface publication benchmark</title>'}))
  await page.goto('http://127.0.0.1:4184/__surface-bench')
  const result = process.env.SURFACE_PUBLICATION_SMOKE_ONLY ? {} : await page.evaluate(async worker => (await import('/__surface-harness.js')).run(`/assets/${worker}`), workers[0])
  assert.deepEqual(errors, [])
  await page.addInitScript(() => {
    const NativeWorker = window.Worker
    window.__selectionSmoke = {requested: false, published: false, terminal: null}
    window.Worker = class extends NativeWorker {
      constructor(...args) {
        super(...args)
        this.addEventListener('message', event => {
          if (['succeeded', 'failed'].includes(event.data?.status)) window.__selectionSmoke.terminal = {status: event.data.status, error: event.data.error}
          if (event.data?.status === 'succeeded' && event.data.meshes?.length) {
            window.__selectionSmoke.published = event.data.meshes.every(mesh => mesh.faceIdsAuthoritative || mesh.faceIdsInferred)
          }
        })
      }
      postMessage(message, ...args) {
        if (message?.type === 'build' && message.selectionSurfaces === true) window.__selectionSmoke.requested = true
        return super.postMessage(message, ...args)
      }
    }
  })
  await page.goto('http://127.0.0.1:4184/')
  try {
    await page.waitForFunction(() => window.__selectionSmoke.requested && window.__selectionSmoke.published, undefined, {timeout: 30000})
  } catch (error) {
    console.error(await page.evaluate(() => ({smoke: window.__selectionSmoke, text: document.body.innerText.slice(0, 5000)})))
    console.error({errors})
    throw error
  }
  const appSmoke = await page.evaluate(() => window.__selectionSmoke)
  await page.waitForFunction(() => document.querySelector('footer.stats strong')?.textContent === '4')
  appSmoke.uiMeshCount = await page.locator('footer.stats strong').first().textContent()
  assert.deepEqual(errors, [])
  const hashes = {}
  for (const file of [`dist/assets/${workers[0]}`, 'public/wasm/geometry-kernel.wasm', 'tools/browser-qualification/surface-group-publication.ts']) {
    hashes[file] = createHash('sha256').update(await readFile(file)).digest('hex')
  }
  await mkdir(output, {recursive: true})
  await writeFile(join(output, 'report.json'), JSON.stringify({browser: browser.version(), hashes, appSmoke, ...result}, null, 2) + '\n')
  console.log(JSON.stringify({appSmoke, reports: result.reports?.map(({fixture, cache, triangles, totalP50, mainSelectionP50}) => ({fixture, cache, triangles, totalP50, mainSelectionP50}))}, null, 2))
} finally { try { await browser?.close() } finally { await server?.close() } }
