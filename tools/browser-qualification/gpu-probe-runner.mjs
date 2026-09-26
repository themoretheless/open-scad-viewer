// Runs an isolated WebGPU probe page and prints the result.
// Usage: node tools/browser-qualification/gpu-probe-runner.mjs [page=probe.html]
// The photo-gpu-probe-runner.mjs and sdf-gpu-probe-runner.mjs files are thin
// wrappers kept for existing invocation points.
import { chromium } from 'playwright'
import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

export async function runGpuProbe(page = 'probe.html') {
  const ROOT = process.env.PROBE_ROOT ?? `${new URL('../..', import.meta.url).pathname}/tmp/browser-gpu`
  const server = createServer(async (req, res) => {
    try {
      const path = join(ROOT, req.url === '/' ? page : req.url.split('?')[0])
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
  const probePage = await browser.newPage()
  probePage.on('console', m => console.log('console:', m.text()))
  probePage.on('pageerror', e => console.log('pageerror:', e.message))
  await probePage.goto('http://127.0.0.1:5298/')
  await probePage.waitForFunction(() => document.getElementById('log').textContent.startsWith('PROBE'), { timeout: 60000 })
  console.log(await probePage.locator('#log').textContent())
  await browser.close()
  server.close()
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runGpuProbe(process.argv[2])
}
