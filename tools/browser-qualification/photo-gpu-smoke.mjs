// Browser smoke for the WebGPU dense sweep: upload shell6 photos, run the
// reconstruction, read the surface vertex count. GPU path: 10,499 vertices;
// CPU fallback: 10,507 (both measured natively on the same inputs).
import { chromium } from 'playwright'
import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { extname, join } from 'node:path'

const REPO = new URL('../..', import.meta.url).pathname
const DIST = process.env.PHOTO_SMOKE_DIST ?? `${REPO}/dist`
// Frozen shell6 inputs converted to PNG (sips -s format png). Not committed.
const PHOTOS = process.env.PHOTO_SMOKE_INPUTS ?? `${REPO}/tmp/browser-gpu/shell6`

const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' }
const server = createServer(async (req, res) => {
  try {
    const path = join(DIST, req.url === '/' ? 'index.html' : req.url.split('?')[0])
    const body = await readFile(path)
    res.writeHead(200, { 'content-type': mime[extname(path)] ?? 'application/octet-stream' })
    res.end(body)
  } catch {
    res.writeHead(404); res.end()
  }
})
await new Promise(resolve => server.listen(5299, resolve))

const HOME = process.env.HOME
const browser = await chromium.launch({
  headless: true,
  executablePath: `${HOME}/Library/Caches/ms-playwright/chromium_headless_shell-1228/chrome-headless-shell-mac-arm64/chrome-headless-shell`,
  args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--headless=new'],
})
const page = await browser.newPage()
const errors = []
page.on('pageerror', e => errors.push(`pageerror: ${e.message}`))
page.on('console', m => { if (m.type() === 'error') errors.push(`console: ${m.text()}`) })

try {
  await page.goto('http://127.0.0.1:5299/')
  // WebGPU in a Worker context check.
  const workerGpu = await page.evaluate(() => new Promise(resolve => {
    const probe = new Worker(URL.createObjectURL(new Blob([
      'onmessage=()=>{postMessage(!!navigator.gpu)}',
    ], { type: 'text/javascript' })))
    probe.onmessage = e => resolve(e.data)
    probe.onerror = () => resolve(false)
    probe.postMessage(null)
    setTimeout(() => resolve('timeout'), 5000)
  }))
  console.log('webgpu-in-worker:', workerGpu)

  await page.locator('details#photogrammetry summary').click()
  const files = ['M97A2474.png','M97A2475.png','M97A2476.png','M97A2477.png','M97A2478.png','M97A2479.png']
    .map(name => join(PHOTOS, name))
  await page.setInputFiles('input[accept="image/jpeg,image/png,image/webp"]', files)
  await page.waitForSelector('.photo-list article', { timeout: 60000 })
  // Focal equivalent 85 mm reproduces the frozen focal of 2266.67 px at 960 px wide.
  const focals = page.locator('.photo-list article input[type=number]')
  for (let i = 0; i < 6; i++) {
    await focals.nth(i).fill('85')
    await focals.nth(i).dispatchEvent('change')
  }
  await page.locator('button', { hasText: /Reconstruct 3D|Восстановить 3D/ }).click()
  // Busy ends when the run completes (button returns); then read the outcome.
  await page.waitForSelector('.photo-stats', { timeout: 240000 })
  await page.waitForSelector('button:not([disabled]) >> text=/Reconstruct 3D|Восстановить 3D/', { timeout: 240000 })
  const stats = await page.locator('.photo-stats').textContent()
  console.log('stats:', stats)
  const message = await page.locator('.photo-error').textContent().catch(() => null)
  const warnings = await page.locator('p[role=status]').allTextContents().catch(() => [])
  console.log('message:', message, '| warnings:', warnings)
  console.log('errors:', errors.length ? errors : 'none')
} finally {
  await browser.close()
  server.close()
}
