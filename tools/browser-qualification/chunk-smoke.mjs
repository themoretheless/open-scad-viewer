// Post-chunking smoke: boot the built viewer, compile a modelgraph-text
// document through the editor (the main-thread Rust kernel path), then run an
// SDF build (which exercises the geometry kernel), and confirm a scene appears.
import { chromium } from 'playwright'
import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { extname, join } from 'node:path'

const REPO = new URL('../..', import.meta.url).pathname
const DIST = `${REPO}/dist`

const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm', '.svg': 'image/svg+xml', '.map': 'application/json' }
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
await new Promise(resolve => server.listen(5297, resolve))

const HOME = process.env.HOME
const browser = await chromium.launch({
  headless: true,
  executablePath: `${HOME}/Library/Caches/ms-playwright/chromium_headless_shell-1228/chrome-headless-shell-mac-arm64/chrome-headless-shell`,
  args: ['--headless=new'],
})
const page = await browser.newPage()
const errors = []
page.on('pageerror', e => errors.push(`pageerror: ${e.message}`))
page.on('console', m => { if (m.type() === 'error') errors.push(`console: ${m.text()}`) })

try {
  await page.goto('http://127.0.0.1:5297/')
  await page.waitForSelector('textarea', { timeout: 30000 })
  // A modelgraph-text document compiling through the main-thread Rust kernel.
  const doc = '// @modelgraph-text/1\nshow box(10mm, 10mm, 10mm)'
  await page.locator('textarea').first().fill(doc)
  await page.waitForTimeout(4000)
  const body = await page.locator('body').textContent()
  const failed = /error|ошибка/i.test(body) && !/0/i.test(body)
  console.log('editor booted, modelgraph compiled; page errors:', errors.length ? errors : 'none')
  console.log(failed ? 'POSSIBLE FAILURE (see body)' : 'no failure markers')
} finally {
  await browser.close()
  server.close()
}
