// Browser benchmark for WebGPU shader variants. It serves a tiny Vite page
// that imports the production compute plumbing, then compares baseline WGSL
// against variant-selected WGSL in a real browser.
import { chromium } from 'playwright'
import { createServer } from 'vite'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { join } from 'node:path'

const REPO = new URL('../..', import.meta.url).pathname
const ROOT = join(REPO, 'tmp', 'webgpu-variants-bench')
const PORT = Number(process.env.WEBGPU_VARIANTS_PORT ?? 5312)
const ITERATIONS = Number(process.env.WEBGPU_VARIANTS_ITERATIONS ?? 20)
const WARMUP = Number(process.env.WEBGPU_VARIANTS_WARMUP ?? 3)
const SDF_SIZES = (process.env.WEBGPU_VARIANTS_SDF_SIZES ?? '32,48,64')
  .split(',')
  .map(value => Number(value.trim()))
  .filter(value => Number.isInteger(value) && value > 0)
const CHROME_EXECUTABLE = process.env.CHROME_EXECUTABLE
if (!SDF_SIZES.length) throw new Error('WEBGPU_VARIANTS_SDF_SIZES must include at least one positive integer')

async function extractSdfWgsl() {
  const source = await readFile(join(REPO, 'crates/sdf-core/src/lib.rs'), 'utf8')
  const match = source.match(/pub const SDF_WGSL: &str = r##"([\s\S]*?)"##;/)
  if (!match) throw new Error('Unable to extract SDF_WGSL from sdf-core')
  return match[1]
}

await mkdir(ROOT, { recursive: true })
await writeFile(join(ROOT, 'sdf.wgsl'), await extractSdfWgsl())
await writeFile(join(ROOT, 'index.html'), '<div id="log">running</div><script type="module" src="/tmp/webgpu-variants-bench/main.ts"></script>\n')
await writeFile(join(ROOT, 'main.ts'), `
import { runGpuCompute } from '/src/services/webgpuCompute.ts'
import { sdfWgslVariants } from '/src/services/sdfGpu.ts'

const iterations = ${JSON.stringify(ITERATIONS)}
const warmup = ${JSON.stringify(WARMUP)}
const sdfSizes = ${JSON.stringify(SDF_SIZES)}
const baseline = await (await fetch('/tmp/webgpu-variants-bench/sdf.wgsl')).text()
const variants = sdfWgslVariants(baseline)
const linear = variants.find(variant => variant.label === 'sdf-linear-indexing')

function sdfDispatch(size) {
  const cells = [size, size, size]
  const [nx, ny, nz] = cells
  const total = (nx + 1) * (ny + 1) * (nz + 1)
  const params = new ArrayBuffer(64)
  const view = new DataView(params)
  ;[nx, ny, nz, 1].forEach((v, i) => view.setUint32(i * 4, v, true))
  ;[-12, -12, -12].forEach((v, i) => view.setFloat32(16 + i * 4, v, true))
  for (let i = 0; i < 3; i++) view.setFloat32(28 + i * 4, 24 / cells[i], true)
  return {
    total,
    dispatch: {
      buffers: [
        { binding: 0, data: params, uniform: true },
        { binding: 1, data: new Uint32Array([0]) },
        { binding: 2, data: new Float32Array([0, 0, 0, 10, 0, 0, 0, 0]) },
        { binding: 3, data: new Uint32Array([0, 0]) },
        { binding: 4, data: new Float32Array([0]) },
        { binding: 5, data: new Float32Array(0), output: true },
      ],
      outputBytes: total * 4,
      workgroups: [Math.ceil(total / 256), 1, 1],
    },
  }
}
async function measure(label, job) {
  const samples = []
  for (let i = 0; i < warmup; i++) await runGpuCompute(job)
  for (let i = 0; i < iterations; i++) {
    const start = performance.now()
    await runGpuCompute(job)
    samples.push(performance.now() - start)
  }
  samples.sort((a, b) => a - b)
  return {
    label,
    minMs: samples[0],
    medianMs: samples[Math.floor(samples.length / 2)],
    maxMs: samples[samples.length - 1],
  }
}
const results = []
for (const size of sdfSizes) {
  const { total, dispatch } = sdfDispatch(size)
  const baselineResult = await measure('sdf-baseline', { wgsl: baseline, entryPoint: 'main', dispatches: [dispatch] })
  const entry = { workload: 'sdf', size, totalSamples: total, baseline: baselineResult }
  if (linear) {
    const linearResult = await measure('sdf-linear-indexing', {
      wgsl: baseline,
      wgslVariants: [linear],
      entryPoint: 'main',
      dispatches: [dispatch],
    })
    entry.linearIndexing = linearResult
    entry.linearMedianRatio = linearResult.medianMs / baselineResult.medianMs
  }
  results.push(entry)
}
document.getElementById('log')!.textContent = 'RESULT ' + JSON.stringify({
  iterations,
  warmup,
  sdfSizes,
  wgslLanguageFeatures: [...(navigator.gpu?.wgslLanguageFeatures ?? [])].sort(),
  results,
})
`)

const server = await createServer({
  root: REPO,
  server: { host: '127.0.0.1', port: PORT, strictPort: true },
  logLevel: 'silent',
})
await server.listen()

const launchOptions = {
  headless: true,
  args: ['--enable-unsafe-webgpu', '--headless=new'],
}
if (CHROME_EXECUTABLE) launchOptions.executablePath = CHROME_EXECUTABLE

const browser = await chromium.launch(launchOptions)
try {
  const page = await browser.newPage()
  page.on('console', message => console.log('console:', message.text()))
  page.on('pageerror', error => console.log('pageerror:', error.message))
  await page.goto(`http://127.0.0.1:${PORT}/tmp/webgpu-variants-bench/index.html`)
  await page.waitForFunction(() => document.getElementById('log')?.textContent?.startsWith('RESULT '), { timeout: 120000 })
  console.log(await page.locator('#log').textContent())
} finally {
  await browser.close()
  await server.close()
}
