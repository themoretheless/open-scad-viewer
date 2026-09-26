// Direct browser benchmark for the photogrammetry WebGPU stages. This
// avoids the app UI and measures the production kernel/worker-adjacent path:
// decode shell6 PNGs, sparse matching (browser WebGPU round trip vs the CPU
// reference on the same session), densePrepare(), then runGpuSweep()
// with baseline WGSL and the kernel-provided linear_indexing variant.
import { chromium } from 'playwright'
import { createServer } from 'vite'
import { access, mkdir, writeFile } from 'node:fs/promises'
import { join } from 'node:path'

const REPO = new URL('../..', import.meta.url).pathname
const ROOT = join(REPO, 'tmp', 'photo-gpu-bench')
const PHOTOS = process.env.PHOTO_SMOKE_INPUTS ?? `${REPO}/tmp/browser-gpu/shell6`
const PORT = Number(process.env.PHOTO_GPU_BENCH_PORT ?? 5299)
const ITERATIONS = Number(process.env.PHOTO_GPU_BENCH_ITERATIONS ?? 5)
const WARMUP = Number(process.env.PHOTO_GPU_BENCH_WARMUP ?? 1)
const RESOLUTION = Number(process.env.PHOTO_GPU_BENCH_RESOLUTION ?? 128)
const CHROME_EXECUTABLE = process.env.CHROME_EXECUTABLE
const PHOTO_NAMES = ['M97A2474.png', 'M97A2475.png', 'M97A2476.png', 'M97A2477.png', 'M97A2478.png', 'M97A2479.png']

for (const name of PHOTO_NAMES) await access(join(PHOTOS, name))

await mkdir(ROOT, { recursive: true })
await writeFile(join(ROOT, 'index.html'), '<div id="log">running</div><script type="module" src="/tmp/photo-gpu-bench/main.ts"></script>\n')
await writeFile(join(ROOT, 'main.ts'), `
import { compilePhotogrammetryKernel } from '/src/services/photogrammetry/module.ts'
import { PhotogrammetryKernel } from '/src/services/photogrammetry/kernel.ts'
import type {PhotoReconstruction} from '/src/services/photogrammetry/kernel.ts'
import { decodePhoto } from '/src/services/photogrammetry/input.ts'
import { runGpuSweep, SWEEP_LINEAR_INDEXING_VARIANT } from '/src/services/photogrammetry/gpuSweep.ts'
import { runGpuMatching } from '/src/services/photogrammetry/gpuMatching.ts'

const photoNames = ${JSON.stringify(PHOTO_NAMES)}
const iterations = ${JSON.stringify(ITERATIONS)}
const warmup = ${JSON.stringify(WARMUP)}
const resolution = ${JSON.stringify(RESOLUTION)}

async function loadPhoto(name: string) {
  const blob = await (await fetch('/tmp/browser-gpu/shell6/' + name)).blob()
  const file = new File([blob], name, { type: 'image/png' })
  return decodePhoto(file, 85, 960)
}

async function measure(label: string, run: () => Promise<Float32Array>) {
  for (let i = 0; i < warmup; i++) await run()
  const samples = []
  let outputLength = 0
  for (let i = 0; i < iterations; i++) {
    const start = performance.now()
    const scores = await run()
    samples.push(performance.now() - start)
    outputLength = scores.length
  }
  samples.sort((a, b) => a - b)
  return {
    label,
    outputLength,
    minMs: samples[0],
    medianMs: samples[Math.floor(samples.length / 2)],
    maxMs: samples[samples.length - 1],
  }
}

const module = await compilePhotogrammetryKernel()
const kernel = new PhotogrammetryKernel(module)
try {
  const decodeStart = performance.now()
  const photos = await Promise.all(photoNames.map(loadPhoto))
  const decodeMs = performance.now() - decodeStart
  for (const photo of photos) kernel.add(photo)
  // Sparse: browser WebGPU matching round trip first (when eligible), then the
  // CPU reference measured on the same session for a direct A/B.
  const matchPrepareStart = performance.now()
  const matchPrepared = kernel.sparsePrepare()
  const sparseGpuPrepareMs = performance.now() - matchPrepareStart
  let sparse: PhotoReconstruction | null = null
  let sparseMatch = null
  let sparseGpuFinishMs = null
  let sparseGpuTotalMs = null
  const gpuStart = performance.now()
  if (matchPrepared) {
    sparseMatch = await measure('photo-matching', () => runGpuMatching(matchPrepared.payload, matchPrepared.wgsl))
    const matchBytes = await runGpuMatching(matchPrepared.payload, matchPrepared.wgsl)
    const finishStart = performance.now()
    sparse = kernel.sparseFinish(matchBytes)
    sparseGpuFinishMs = performance.now() - finishStart
    sparseGpuTotalMs = performance.now() - gpuStart
  }
  let sparseCpuMs: number
  if (!sparse) {
    const cpuStart = performance.now()
    sparse = kernel.sparse()
    sparseCpuMs = performance.now() - cpuStart
  } else {
    // The cameras came from the GPU round trip; measure the CPU reference
    // separately on the same session for a direct A/B.
    const cpuStart = performance.now()
    kernel.sparse()
    sparseCpuMs = performance.now() - cpuStart
  }
  const sparseMs = sparseGpuTotalMs ?? sparseCpuMs
  const prepared = kernel.densePrepare(resolution)
  if (!prepared) throw new Error('densePrepare returned null')
  const linear = prepared.wgslVariants?.find(variant => variant.label === SWEEP_LINEAR_INDEXING_VARIANT)
  const baseline = await measure('photo-baseline', () => runGpuSweep(prepared.payload, prepared.wgsl))
  const linearIndexing = linear
    ? await measure('photo-linear-indexing', () => runGpuSweep(prepared.payload, prepared.wgsl, [linear]))
    : null
  document.getElementById('log')!.textContent = 'RESULT ' + JSON.stringify({
    iterations,
    warmup,
    resolution,
    decodeMs,
    sparseMs,
    sparsePath: matchPrepared ? 'gpu' : 'cpu',
    sparseGpuPrepareMs: matchPrepared ? sparseGpuPrepareMs : null,
    sparseGpuFinishMs,
    sparseGpuTotalMs,
    sparseCpuMs,
    sparseMatch,
    cameras: sparse.cameras.length,
    points: sparse.positions.length / 3,
    wgslLanguageFeatures: [...(navigator.gpu?.wgslLanguageFeatures ?? [])].sort(),
    variants: prepared.wgslVariants?.map(variant => variant.label) ?? [],
    baseline,
    linearIndexing,
    linearMedianRatio: linearIndexing ? linearIndexing.medianMs / baseline.medianMs : null,
  })
} finally {
  kernel.clear()
}
`)

const server = await createServer({
  root: REPO,
  server: { host: '127.0.0.1', port: PORT, strictPort: true },
  logLevel: 'silent',
})
await server.listen()

const launchOptions = {
  headless: true,
  args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--headless=new'],
}
if (CHROME_EXECUTABLE) launchOptions.executablePath = CHROME_EXECUTABLE

const browser = await chromium.launch(launchOptions)
try {
  const page = await browser.newPage()
  page.on('console', message => console.log('console:', message.text()))
  page.on('pageerror', error => console.log('pageerror:', error.message))
  await page.goto(`http://127.0.0.1:${PORT}/tmp/photo-gpu-bench/index.html`)
  const workerGpu = await page.evaluate(() => new Promise(resolve => {
    const probe = new Worker(URL.createObjectURL(new Blob([
      'onmessage=()=>{postMessage(!!navigator.gpu)}',
    ], { type: 'text/javascript' })))
    probe.onmessage = event => resolve(event.data)
    probe.onerror = () => resolve(false)
    probe.postMessage(null)
    setTimeout(() => resolve('timeout'), 5000)
  }))
  console.log('webgpu-in-worker:', workerGpu)
  await page.waitForFunction(() => document.getElementById('log')?.textContent?.startsWith('RESULT '), { timeout: 240000 })
  console.log(await page.locator('#log').textContent())
} finally {
  await browser.close()
  await server.close()
}
