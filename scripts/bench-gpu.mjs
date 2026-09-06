#!/usr/bin/env node
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { cpus, platform, arch, release, totalmem } from 'node:os'
import { mkdir, readFile, writeFile, readdir } from 'node:fs/promises'
import { dirname, join, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { build, preview } from 'vite'
import { loadQualificationPlaywrightPackage } from './qualificationPlaywrightPackage.mjs'

const root = fileURLToPath(new URL('../', import.meta.url))
const args = process.argv.slice(2)
const value = (name, fallback) => { const index = args.indexOf(name); return index < 0 ? fallback : args[index + 1] }
const positive = (name, fallback, max = 10_000) => {
  const number = Number(value(name, fallback))
  if (!Number.isSafeInteger(number) || number < 1 || number > max) throw new Error(`${name} must be an integer in [1, ${max}]`)
  return number
}
if (args.includes('--help')) {
  console.log('Usage: node scripts/bench-gpu.mjs [--out DIR] [--frames 30] [--warmup 8] [--uploads 8] [--triangles 5000,50000,200000] [--idle-ms 500] [--headless] [--executable PATH] [--no-timestamps] [--heap-sampling] [--heap-frames 180] [--renderer-source PATH] [--fps] [--fps-diagnostics] [--diagnostics-only]\nUses the existing isolated Playwright package; never installs dependencies. Defaults to headed bundled Chromium and records actual adapter metadata. Timings must run without other benchmark loads. Use --fps for continuous frame submission measurements without per-frame GPU fences; --fps-diagnostics adds a separate GPU timestamp/counter pass and visual state checks. Optional CDP heap sampling is a separate pass after timings, without renderer instrumentation wrappers. --renderer-source loads a saved renderer snapshot at the original module id for A/B comparison without editing product files; the override and resulting source-map binding are recorded.')
  process.exit(0)
}
const allowed = new Set(['--out', '--frames', '--warmup', '--uploads', '--triangles', '--idle-ms', '--headless', '--executable', '--no-timestamps', '--heap-sampling', '--heap-frames', '--renderer-source', '--fps', '--fps-diagnostics', '--diagnostics-only'])
for (let index = 0; index < args.length; index++) {
  if (!allowed.has(args[index])) throw new Error(`Unknown option ${args[index]}`)
  if (!['--headless', '--no-timestamps', '--heap-sampling', '--fps', '--fps-diagnostics', '--diagnostics-only'].includes(args[index])) { if (!args[index + 1] || args[index + 1].startsWith('--')) throw new Error(`Missing value for ${args[index]}`); index++ }
}
const options = {
  triangles: String(value('--triangles', '5000,50000,200000')).split(',').map(Number),
  frames: positive('--frames', 30, 1_000), warmup: positive('--warmup', 8, 100),
  uploads: positive('--uploads', 8, 100), idleMs: positive('--idle-ms', 500, 10_000), timestamps: !args.includes('--no-timestamps'),
}
if (args.includes('--fps') && options.frames < 2) throw new Error('--fps requires at least 2 measured frames')
const heapFrames = positive('--heap-frames', 180, 3_600)
if (!options.triangles.length || options.triangles.length > 10 || options.triangles.some(n => !Number.isSafeInteger(n) || n < 100 || n > 750_000)) throw new Error('--triangles requires 1-10 integer counts in [100,750000]')
const output = resolve(root, value('--out', `tmp/performance/gpu-${new Date().toISOString().replaceAll(':', '-')}`))
const git = (...params) => execFileSync('git', params, { cwd: root, encoding: 'utf8' }).trim()
const sha256 = data => createHash('sha256').update(data).digest('hex')
async function sourceEvidence() {
  const files = [...new Set(git('ls-files', '--cached', '--others', '--exclude-standard', '-z', '--', 'src', 'benchmarks', 'scripts', 'package.json', 'package-lock.json', 'tools/browser-qualification/package.json', 'tools/browser-qualification/package-lock.json').split('\0').filter(Boolean))].sort()
  const hashes = []
  for (const file of files) {
    try { hashes.push({ path: file, sha256: sha256(await readFile(join(root, file))) }) }
    catch (error) { if (error.code === 'ENOENT') hashes.push({ path: file, deleted: true }); else throw error }
  }
  return { head: git('rev-parse', 'HEAD'), status: git('status', '--porcelain=v1'), trackedDiffSha256: sha256(execFileSync('git', ['diff', 'HEAD', '--binary'], { cwd: root })), files: hashes, sourceSha256: sha256(JSON.stringify(hashes)) }
}
async function fileHashes(directory) {
  const entries = []
  for (const item of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, item.name)
    if (item.isDirectory()) entries.push(...await fileHashes(path))
    else entries.push({ path: relative(output, path), sha256: sha256(await readFile(path)) })
  }
  return entries
}
function summarizeHeap(profile) {
  const sites = []
  let sampledEstimatedBytes = 0, rendererStackEstimatedBytes = 0
  const walk = (node, rendererStack = false) => {
    rendererStack ||= ['render', 'setSection'].includes(node.callFrame.functionName)
    sampledEstimatedBytes += node.selfSize
    if (rendererStack) rendererStackEstimatedBytes += node.selfSize
    if (node.selfSize) sites.push({ estimatedBytes: node.selfSize, rendererStack, functionName: node.callFrame.functionName || '(anonymous)', url: node.callFrame.url, line: node.callFrame.lineNumber + 1, column: node.callFrame.columnNumber + 1 })
    for (const child of node.children) walk(child, rendererStack)
  }
  walk(profile.head)
  return { sampledEstimatedBytes, rendererStackEstimatedBytes, samples: profile.samples.length, topSites: sites.sort((a, b) => b.estimatedBytes - a.estimatedBytes).slice(0, 30) }
}
async function captureHeapProfiles(context, page) {
  const cdp = await context.newCDPSession(page)
  const cases = []
  await cdp.send('HeapProfiler.enable')
  try {
    for (const mode of ['shaded', 'edges', 'xray']) for (const activity of ['camera', 'scan-plane']) {
      const metadata = await page.evaluate(mode => window.gpuBenchmark.prepareHeapWorkload(mode), mode)
      await page.evaluate(activity => window.gpuBenchmark.runHeapWorkload(activity, 30), activity)
      await cdp.send('HeapProfiler.collectGarbage')
      await cdp.send('HeapProfiler.startSampling', { samplingInterval: 4096, stackDepth: 128, includeObjectsCollectedByMajorGC: true, includeObjectsCollectedByMinorGC: true })
      await page.evaluate(({ activity, frames }) => window.gpuBenchmark.runHeapWorkload(activity, frames), { activity, frames: heapFrames })
      const { profile } = await cdp.send('HeapProfiler.stopSampling')
      const file = `heap-${mode}-${activity}.heapprofile`
      await writeFile(join(output, file), JSON.stringify(profile) + '\n')
      cases.push({ ...metadata, activity, frames: heapFrames, profile: file, ...summarizeHeap(profile) })
      await page.evaluate(() => window.gpuBenchmark.disposeHeapWorkload())
    }
    return {
      samplingIntervalBytes: 4096, includeObjectsCollectedByMajorGC: true, includeObjectsCollectedByMinorGC: true, cases,
      methodology: [
        'Separate untimed CDP sampling pass on a fresh uninstrumented renderer/device; GPU counter/timestamp wrappers are absent.',
        '128 instances share a 4900-triangle indexed sphere. Scene preparation, upload, idle edge warmup and 30 warmup frames precede sampling.',
        'V8 uses probabilistic allocation sampling. Reported bytes are estimates, not exact object counts, peak memory or retained heap.',
        'Collected objects are included so short-lived allocation churn remains observable; totals also include the fixture/browser scheduler.',
        'rendererStackEstimatedBytes includes allocations below render() or setSection() stacks. Generated bundle source maps are saved next to .heapprofile artifacts.',
      ],
    }
  } finally {
    await page.evaluate(() => window.gpuBenchmark.disposeHeapWorkload()).catch(() => {})
    await cdp.detach()
  }
}
await mkdir(dirname(output), { recursive: true })
await mkdir(output)
const source = await sourceEvidence()
const rendererPath = join(root, 'src/services/webgpuRenderer.ts')
const overridePath = value('--renderer-source') ? resolve(root, value('--renderer-source')) : null
const rendererBytes = await readFile(overridePath ?? rendererPath)
const rendererSource = { module: 'src/services/webgpuRenderer.ts', overridePath, sha256: sha256(rendererBytes), effectiveSourceSha256: sha256(JSON.stringify(source.files.map(file => file.path === 'src/services/webgpuRenderer.ts' ? { path: file.path, sha256: sha256(rendererBytes) } : file))) }
await writeFile(join(output, 'renderer-source.ts'), rendererBytes)
let browser, server
const errors = []
try {
  const { playwright, packageMetadata } = await loadQualificationPlaywrightPackage()
  const bundleDir = join(output, 'bundle')
  console.log(`Building benchmark from ${source.head.slice(0, 12)} (${source.sourceSha256.slice(0, 12)})`)
  await build({ configFile: false, root, base: './', logLevel: 'warn', plugins: [{ name: 'benchmark-renderer-snapshot', enforce: 'pre', load(id) { return id === rendererPath ? rendererBytes.toString('utf8') : null } }], build: { outDir: bundleDir, emptyOutDir: false, sourcemap: true, rollupOptions: { input: join(root, 'benchmarks/gpu.html') } } })
  const bundle = await fileHashes(bundleDir)
  const rendererBindings = []
  for (const file of bundle.filter(file => file.path.endsWith('.map'))) {
    const map = JSON.parse(await readFile(join(output, file.path), 'utf8'))
    for (let index = 0; index < map.sources.length; index++) if (map.sources[index].endsWith('/src/services/webgpuRenderer.ts')) rendererBindings.push({ sourceMap: file.path, sha256: sha256(map.sourcesContent[index]) })
  }
  if (rendererBindings.length !== 1 || rendererBindings[0].sha256 !== rendererSource.sha256) throw new Error('Built renderer source-map content does not match the requested immutable renderer snapshot')
  rendererSource.bundleBinding = rendererBindings[0]
  server = await preview({ configFile: false, root, logLevel: 'warn', build: { outDir: bundleDir }, preview: { host: '127.0.0.1', port: 0, strictPort: true, headers: { 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp' } } })
  const origin = server.resolvedUrls.local[0]
  const launch = { headless: args.includes('--headless'), args: ['--disable-background-timer-throttling', '--disable-renderer-backgrounding', '--disable-backgrounding-occluded-windows'], ...(value('--executable') ? { executablePath: value('--executable') } : {}) }
  browser = await playwright.chromium.launch(launch)
  const browserVersion = browser.version()
  const context = await browser.newContext({ viewport: { width: 980, height: 720 }, deviceScaleFactor: 1 })
  await context.route('**/*', route => { const url = route.request().url(); return url.startsWith(origin) || url.startsWith('data:') || url.startsWith('blob:') ? route.continue() : route.abort() })
  const page = await context.newPage()
  page.on('pageerror', error => errors.push(error.message))
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()) })
  await page.exposeFunction('captureBenchmarkFrame', name => page.screenshot({ path: join(output, name) }))
  await page.goto(new URL('benchmarks/gpu.html', origin).href)
  await page.waitForFunction(() => Boolean(window.gpuBenchmark), { timeout: 30_000 })
  console.log(`Running native WebGPU workloads in Chromium ${browserVersion}; ${options.frames} measured frames per case`)
  const result = await Promise.race([
    page.evaluate(({ options, fps, diagnosticsOnly }) => diagnosticsOnly ? window.gpuBenchmark.runFpsDiagnostics(options) : fps ? window.gpuBenchmark.runFps(options) : window.gpuBenchmark.run(options), { options, fps: args.includes('--fps'), diagnosticsOnly: args.includes('--diagnostics-only') }),
    new Promise((_, reject) => { const timer = setTimeout(() => reject(new Error('GPU benchmark exceeded 600 seconds')), 600_000); timer.unref() }),
  ])
  await writeFile(join(output, 'timing-results.json'), JSON.stringify({ ...result, source, rendererSource, bundle, browserVersion }, null, 2) + '\n')
  await page.screenshot({ path: join(output, 'viewport.png') })
  await page.evaluate(() => window.gpuBenchmark.dispose())
  const heapSampling = args.includes('--heap-sampling') ? await captureHeapProfiles(context, page) : null
  const fpsDiagnostics = args.includes('--fps-diagnostics') ? await page.evaluate(options => window.gpuBenchmark.runFpsDiagnostics(options), options) : null
  const finalSource = await sourceEvidence()
  const sourceChangedDuringRun = source.sourceSha256 !== finalSource.sourceSha256
  const report = { ...result, host: { platform: platform(), arch: arch(), osRelease: release(), cpu: cpus()[0]?.model, logicalCpus: cpus().length, totalMemoryBytes: totalmem(), node: process.version }, browser: { version: browserVersion, launch, packageMetadata }, source, rendererSource, sourceChangedDuringRun, ...(sourceChangedDuringRun ? { sourceAfter: finalSource } : {}), bundle, browserErrors: errors, heapSampling, fpsDiagnostics }
  await writeFile(join(output, 'results.json'), JSON.stringify(report, null, 2) + '\n')
  const lines = ['kind\ttriangles/entities\tmode/activity\tCPU p50 ms\tGPU pass p50 ms\tqueue completion p50 ms\tsubmitted FPS\tframe interval p95 ms']
  for (const scenario of report.scenarios) {
    lines.push([scenario.kind, scenario.triangles ?? scenario.entities ?? scenario.requestedTriangles, [scenario.mode, scenario.activity].filter(Boolean).join('/'), (scenario.setMeshesCpuMs?.median ?? scenario.renderCpuMs?.median ?? scenario.coldSetMeshesCpuMs)?.toFixed(3) ?? '', scenario.renderPassGpuMs?.median?.toFixed(3) ?? '', scenario.queueCompletionMs?.median?.toFixed(3) ?? '', scenario.submittedFps?.toFixed(1) ?? '', scenario.frameIntervalMs?.p95?.toFixed(3) ?? ''].join('\t'))
  }
  await writeFile(join(output, 'summary.tsv'), lines.join('\n') + '\n')
  console.log(lines.join('\n'))
  console.log(`Evidence: ${join(output, 'results.json')}`)
  if (sourceChangedDuringRun) console.warn('Source changed during benchmark. The executed immutable bundle hashes are recorded; do not pair these numbers with the final working tree.')
  if (errors.length || result.errors.length) process.exitCode = 1
} catch (error) {
  await writeFile(join(output, 'failure.json'), JSON.stringify({ schema: 'open-scad-viewer-gpu-benchmark-failure-v1', failedAt: new Date().toISOString(), error: error instanceof Error ? error.stack : String(error), source, rendererSource, options, browserErrors: errors }, null, 2) + '\n')
  console.error(error)
  process.exitCode = 1
} finally {
  await browser?.close()
  await new Promise(resolve => server ? server.httpServer.close(resolve) : resolve())
}
