// Runs the isolated WebGPU sweep probe page and prints the result.
import { chromium } from 'playwright'
import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { join } from 'node:path'

const ROOT = process.env.PROBE_ROOT ?? `${new URL('../..', import.meta.url).pathname}/tmp/browser-gpu`
const server = createServer(async (req, res) => {
  try {
    const path = join(ROOT, req.url === '/' ? 'probe.html' : req.url.split('?')[0])
    const body = await readFile(path)
    res.writeHead(200, { 'content-type': path.endsWith('.wgsl') ? 'text/plain' : path.endsWith('.js') ? 'text/javascript' : 'text/html' })
    res.end(body)
  } catch {
    res.writeHead(404); res.end()
  }
})
await new Promise(resolve => server.listen(5298, resolve))
const HOME = process.env.HOME
const browser = await chromium.launch({
  headless: true,
  executablePath: `${HOME}/Library/Caches/ms-playwright/chromium_headless_shell-1228/chrome-headless-shell-mac-arm64/chrome-headless-shell`,
  args: ['--enable-unsafe-webgpu', '--headless=new'],
})
const page = await browser.newPage()
page.on('console', m => console.log('console:', m.text()))
page.on('pageerror', e => console.log('pageerror:', e.message))
await page.goto('http://127.0.0.1:5298/')
await page.waitForFunction(() => document.getElementById('log').textContent.startsWith('PROBE'), { timeout: 60000 })
console.log(await page.locator('#log').textContent())
await browser.close()
server.close()
