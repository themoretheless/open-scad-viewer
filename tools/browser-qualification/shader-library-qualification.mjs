// Visual qualification of the shader library on a real GPU. Serves a tiny
// Vite page that drives the production WebGPURenderer through every shading
// model, matcap capture, environment map, theme, shadow, and section-cap
// combination, then asserts pipeline creation and non-blank output per
// configuration. Screenshots and a JSON/markdown report land in output/.
import { chromium } from 'playwright'
import { createServer } from 'vite'
import { mkdir, writeFile } from 'node:fs/promises'
import { join } from 'node:path'

const REPO = new URL('../..', import.meta.url).pathname
const ROOT = join(REPO, 'tmp', 'shader-library-qual')
const OUT = join(REPO, 'output', 'shader-library-qualification')
const PORT = Number(process.env.SHADER_QUAL_PORT ?? 5314)
const CHROME_EXECUTABLE = process.env.CHROME_EXECUTABLE

const PAGE_TS = `
import { WebGPURenderer } from '/src/services/webgpuRenderer.ts'
import { buildMeshBvh } from '/src/services/meshBvh.ts'
import { extractSemanticEdges } from '/src/services/meshTopology.ts'
import { geometryAssetId } from '/src/core/scene.ts'
import { getShader, immediateObjectShader, instancedObjectShader } from '/src/services/shaders/index.ts'

const SHADERS = ['mesh', 'meshPbr', 'meshMatcap', 'meshToon', 'meshUnlit']
const VARIANTS = ['uniform', 'immediate', 'instanced']

// In-memory uniformity repair, applied only with ?patched=1. Historical
// context: the WGSL once failed Tint's uniformity analysis — shadowFactor
// early-returned on non-uniform bounds before textureSampleCompare, and the
// mesh-family fragment shaders called shadowFactor after the section-cap
// early return. The fix has landed in crates/raster-core/shaders (clamped
// coordinates + select, hoisted call); this patch is kept as a tripwire so a
// regression shows up as stock-FAIL/patched-PASS. Nothing on disk is modified.
function patchSource(source) {
  let out = source
  out = out.replaceAll(
    'if (ndc.z < 0.0 || ndc.z > 1.0 || uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) { return 1.0; }',
    'let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;')
  out = out.replaceAll('shadowSampler, uv + vec2f', 'shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f')
  out = out.replaceAll('return mix(1.0, sum / 9.0, sc.shadowParams.w);', 'return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);')
  // Hoist the shadowFactor call above the section-cap early return.
  const capReturn = 'if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return'
  const call = '  let shadow = shadowFactor(v.w);'
  if (out.includes(call) && out.includes(capReturn)) {
    out = out.replace(call + String.fromCharCode(10), '')
    out = out.replace(capReturn, call + String.fromCharCode(10) + capReturn)
  }
  return out
}
if (new URLSearchParams(location.search).get('patched') === '1') {
  for (const id of ['mesh', 'meshPbr', 'meshMatcap', 'meshToon', 'grid']) {
    const spec = getShader(id)
    spec.source = patchSource(spec.source)
  }
}

function sphere(radius, segments) {
  const positions = []
  const idx = []
  for (let y = 0; y <= segments; y++) {
    const v = y / segments, phi = v * Math.PI
    for (let x = 0; x <= segments; x++) {
      const u = x / segments, theta = u * 2 * Math.PI
      const nx = Math.sin(phi) * Math.cos(theta), ny = Math.cos(phi), nz = Math.sin(phi) * Math.sin(theta)
      positions.push(radius * nx, radius * ny, radius * nz)
      idx.push([nx, ny, nz])
    }
  }
  const indices = []
  for (let y = 0; y < segments; y++) for (let x = 0; x < segments; x++) {
    const a = y * (segments + 1) + x, b = a + segments + 1
    indices.push(a, b, a + 1, b, b + 1, a + 1)
  }
  const vertices = new Float32Array(indices.length * 6)
  const out = new Uint32Array(indices.length)
  for (let t = 0; t < indices.length; t++) {
    const i = indices[t]
    out[t] = t
    vertices.set([positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2], ...idx[i]], t * 6)
  }
  return { vertices, indices: out }
}

function box(size) {
  const h = size / 2
  const faces = [
    [[1, 0, 0], [[h, -h, -h], [h, h, -h], [h, h, h], [h, -h, h]]],
    [[-1, 0, 0], [[-h, -h, h], [-h, h, h], [-h, h, -h], [-h, -h, -h]]],
    [[0, 1, 0], [[-h, h, -h], [-h, h, h], [h, h, h], [h, h, -h]]],
    [[0, -1, 0], [[-h, -h, h], [-h, -h, -h], [h, -h, -h], [h, -h, h]]],
    [[0, 0, 1], [[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h]]],
    [[0, 0, -1], [[h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h]]],
  ]
  const vertices = new Float32Array(6 * 6 * 6)
  const indices = new Uint32Array(6 * 6)
  let v = 0, t = 0
  for (const [normal, corners] of faces) {
    for (const order of [0, 1, 2, 0, 2, 3]) {
      indices[t++] = v
      vertices.set([...corners[order], ...normal], v * 6)
      v++
    }
  }
  return { vertices, indices }
}

function translate(dx, dy, dz) {
  return new Float32Array([1, 0, 0, dx, 0, 1, 0, dy, 0, 0, 1, dz, 0, 0, 0, 1])
}

function makeMesh(geo, transform, color, material) {
  const edges = extractSemanticEdges(geo.vertices, geo.indices)
  return {
    vertices: geo.vertices,
    indices: geo.indices,
    geometryAssetId: geometryAssetId(geo.vertices, geo.indices),
    bvh: buildMeshBvh(geo.vertices, geo.indices),
    edgeIndices: edges.indices,
    topology: edges.diagnostics,
    color,
    transform,
    faceIds: new Uint32Array(geo.indices.length / 3),
    provenance: [{ triangleStart: 0, triangleEnd: geo.indices.length / 3, source: null, backside: false }],
    ...(material ? { material } : {}),
  }
}

const canvas = document.getElementById('view')
canvas.width = 640
canvas.height = 480
const renderer = new WebGPURenderer()
const ok = await renderer.init(canvas)
if (!ok) throw new Error('WebGPURenderer.init failed')

// GPUAdapterInfo exposes its fields as prototype getters: spreading it yields {}.
const adapter = await navigator.gpu.requestAdapter()
const info = adapter?.info
const adapterInfo = info
  ? { vendor: info.vendor, architecture: info.architecture, device: info.device, description: info.description }
  : null

const ball = sphere(10, 32)
const cube = box(16)
const scene = [
  makeMesh(ball, translate(-12, 0, 0), [0.85, 0.67, 0.3, 1]),
  makeMesh(cube, translate(12, 0, 0), [0.35, 0.55, 0.85, 1]),
]
// Instanced path: same asset published several times with distinct transforms.
const instancedScene = [0, 1, 2, 3].map(i => ({
  ...makeMesh(ball, translate((i - 1.5) * 14, 0, 0), [0.85, 0.67, 0.3, 1]),
  entityId: 'entity:inst/' + i,
}))
scene.forEach((m, i) => { m.entityId = 'entity:main/' + i })

let frameResolve = null
renderer.onFrameSubmitted = () => { frameResolve?.() }
window.__statusEvents = []
// GPU errors per configuration: a frame that fails validation is dropped while
// the canvas keeps the previous image, so pixels alone would pass it. The
// renderer reports dropped frames (per-frame validation scope) as 'error'
// status events; uncaptured device errors are collected below.
const gpuErrors = []
renderer.onStatusChange = event => {
  window.__statusEvents.push(JSON.stringify(event, (key, value) => value instanceof Error ? value.message + ' | ' + value.stack : value))
  console.log('status:', window.__statusEvents[window.__statusEvents.length - 1])
  if (event.status === 'error' || event.status === 'device-lost') {
    gpuErrors.push(String(event.error?.message ?? event.message ?? event.status).split(String.fromCharCode(10))[0])
  }
}
function frame() {
  return new Promise(resolve => {
    frameResolve = resolve
    renderer.requestRender()
    setTimeout(resolve, 800)
  }).then(() => { frameResolve = null })
}

renderer.setMeshes(scene)
await frame()

const stats = document.createElement('canvas')
stats.width = canvas.width
stats.height = canvas.height
const sctx = stats.getContext('2d', { willReadFrequently: true })
function canvasStats() {
  sctx.drawImage(canvas, 0, 0)
  const data = sctx.getImageData(0, 0, stats.width, stats.height).data
  const bg = [data[0], data[1], data[2]]
  let sum = 0, sum2 = 0, nonBg = 0
  const n = stats.width * stats.height
  for (let i = 0; i < data.length; i += 4) {
    const lum = 0.2126 * data[i] + 0.7152 * data[i + 1] + 0.0722 * data[i + 2]
    sum += lum
    sum2 += lum * lum
    if (Math.abs(data[i] - bg[0]) + Math.abs(data[i + 1] - bg[1]) + Math.abs(data[i + 2] - bg[2]) > 12) nonBg++
  }
  const mean = sum / n
  return { mean, stddev: Math.sqrt(Math.max(0, sum2 / n - mean * mean)), nonBgFraction: nonBg / n }
}

// Pipeline creation matrix: every shading model x every variant. Dawn reports
// shader/pipeline errors asynchronously, so each creation is wrapped in a
// validation error scope. Modules may already be cached from init, so the
// authoritative compile check is the per-variant createShaderModule loop.
const pipelines = {}
const device = renderer.dev
// The immediate address space is a device feature; its variants are
// 'unsupported' (not failures) where the adapter lacks it (e.g. SwiftShader).
const immediateActive = !!renderer.pipelines.immediateObjectStyle
// Attribute async Dawn validation errors (e.g. bind-group mismatches at draw
// time) to the config that encoded them; run() sets __currentConfig.
window.__currentConfig = 'init'
device.addEventListener('uncapturederror', event => {
  const message = event.error.message.split(String.fromCharCode(10))[0]
  console.log('uncaptured [' + window.__currentConfig + ']: ' + message)
  gpuErrors.push(message)
})
for (const id of SHADERS) {
  for (const variant of VARIANTS) {
    const key = id + '/' + variant
    try {
      device.pushErrorScope('validation')
      const pipeline = renderer.getRenderPipeline(id, { variant })
      const scopeError = await device.popErrorScope()
      if (scopeError) { pipelines[key] = 'INVALID: ' + scopeError.message.split(String.fromCharCode(10))[0]; continue }
      pipelines[key] = 'ok'
    } catch (error) {
      try { await device.popErrorScope() } catch { /* scope already popped */ }
      pipelines[key] = /does not support/.test(String(error)) ? 'unsupported' : String(error)
    }
  }
}
// Compile diagnostics per shader x variant, built straight from the registry
// source (cache-independent): validation scope + compilationInfo.
const shaderDiagnostics = {}
for (const id of SHADERS) {
  const spec = getShader(id)
  for (const variant of VARIANTS) {
    const key = id + '/' + variant
    if (variant === 'immediate' && !immediateActive) {
      shaderDiagnostics[key] = ['unsupported: device lacks the immediate address space']
      continue
    }
    let source = spec.source
    try {
      if (variant === 'immediate') source = immediateObjectShader(source)
      else if (variant === 'instanced') source = instancedObjectShader(source, spec.vertexLayout === 'edge' ? 'EdgeV' : 'V')
    } catch (error) {
      shaderDiagnostics[key] = ['unsupported: ' + String(error)]
      continue
    }
    device.pushErrorScope('validation')
    const module = device.createShaderModule({ code: source })
    const scopeError = await device.popErrorScope()
    const info = await module.getCompilationInfo()
    const errors = info.messages.filter(m => m.type === 'error').map(m => ':' + m.lineNum + ':' + m.linePos + ' ' + m.message)
    shaderDiagnostics[key] = scopeError ? ['INVALID: ' + scopeError.message.split(String.fromCharCode(10))[0]]
      : errors.length ? errors : 'ok'
  }
}

window.__qual = {
  adapterInfo,
  pipelines,
  shaderDiagnostics,
  statusEvents: window.__statusEvents,
  immediateActive: immediateActive,
  async run(config) {
    // Settle the previous configuration, then count only this one's errors.
    await frame()
    gpuErrors.length = 0
    window.__currentConfig = config.name
    renderer.setSection(false, [0, 0, 1], 0)
    renderer.setDisplayMode('shaded')
    renderer.setShadowsEnabled(false)
    renderer.setTheme('default')
    renderer.setBackgroundColor([0.09, 0.09, 0.11])
    renderer.setDefaultShadingModel('phong')
    renderer.setDefaultMaterial({})
    await renderer.setMatcapTexture('procedural')
    await renderer.setEnvMap('none')
    if (config.instanced) renderer.setMeshes(instancedScene)
    else renderer.setMeshes(scene)
    if (config.theme) renderer.setTheme(config.theme)
    if (config.model) renderer.setDefaultShadingModel(config.model)
    if (config.material) renderer.setDefaultMaterial(config.material)
    if (config.matcap) await renderer.setMatcapTexture(config.matcap)
    if (config.env) await renderer.setEnvMap(config.env)
    if (config.shadows) renderer.setShadowsEnabled(true)
    if (config.section) renderer.setSection(true, [0, 0, 1], 0)
    if (config.displayMode) renderer.setDisplayMode(config.displayMode)
    await frame()
    await frame()
    return [...new Set(gpuErrors)]
  },
  // Pixel stats of a compositor screenshot (data URL), decoded via <img> so
  // the result reflects what a user would see rather than drawImage on the
  // WebGPU canvas, which can read back empty after present.
  async imageStats(dataUrl) {
    const img = new Image()
    await new Promise((resolve, reject) => { img.onload = resolve; img.onerror = reject; img.src = dataUrl })
    stats.width = img.width
    stats.height = img.height
    sctx.drawImage(img, 0, 0)
    const data = sctx.getImageData(0, 0, stats.width, stats.height).data
    const bg = [data[0], data[1], data[2]]
    let sum = 0, sum2 = 0, nonBg = 0
    const n = stats.width * stats.height
    for (let i = 0; i < data.length; i += 4) {
      const lum = 0.2126 * data[i] + 0.7152 * data[i + 1] + 0.0722 * data[i + 2]
      sum += lum
      sum2 += lum * lum
      if (Math.abs(data[i] - bg[0]) + Math.abs(data[i + 1] - bg[1]) + Math.abs(data[i + 2] - bg[2]) > 12) nonBg++
    }
    const mean = sum / n
    return { mean, stddev: Math.sqrt(Math.max(0, sum2 / n - mean * mean)), nonBgFraction: nonBg / n, background: bg }
  },
}
document.getElementById('log').textContent = 'READY'
`

const CONFIGS = [
  ...['phong', 'pbr', 'matcap', 'toon', 'unlit'].map(model => ({ name: `model-${model}`, model })),
  { name: 'model-pbr-metal', model: 'pbr', material: { metallic: 1, roughness: 0.3 } },
  ...['procedural', 'studio', 'clay', 'chrome', 'pearl'].map(matcap => ({ name: `matcap-${matcap}`, model: 'matcap', matcap })),
  ...['none', 'studio-softbox', 'outdoor-sky', 'workshop'].map(env => ({ name: `env-${env}`, model: 'pbr', env })),
  { name: 'shadows-off', model: 'phong' },
  { name: 'shadows-on', model: 'phong', shadows: true },
  { name: 'shadows-on-pbr-env', model: 'pbr', env: 'studio-softbox', shadows: true },
  ...['default', 'dark-contrast', 'light'].map(theme => ({ name: `theme-${theme}`, theme })),
  { name: 'section-off', model: 'phong' },
  { name: 'section-on', model: 'phong', section: true },
  { name: 'section-on-toon', model: 'toon', section: true },
  // Section caps after surfaces whose group(2) is the matcap/env group.
  { name: 'section-on-matcap', model: 'matcap', section: true },
  { name: 'section-on-pbr', model: 'pbr', section: true },
  { name: 'edges-mode', model: 'phong', displayMode: 'edges' },
  { name: 'edges-mode-shadows', model: 'phong', displayMode: 'edges', shadows: true },
  { name: 'instanced-phong', model: 'phong', instanced: true },
  { name: 'instanced-toon', model: 'toon', instanced: true },
]

await mkdir(ROOT, { recursive: true })
await mkdir(OUT, { recursive: true })
await writeFile(join(ROOT, 'index.html'), '<base href="/"><canvas id="view"></canvas><div id="log">running</div><script type="module" src="/tmp/shader-library-qual/main.ts"></script>\n')
await writeFile(join(ROOT, 'main.ts'), PAGE_TS)

const server = await createServer({
  root: REPO,
  server: { host: '127.0.0.1', port: PORT, strictPort: true },
  logLevel: 'silent',
})
await server.listen()

// Headless Chrome (shell or full) loses the WebGPU device on canvas present
// in this environment ("A valid external Instance reference no longer
// exists"), so qualification runs headed with the window parked offscreen.
// SHADER_QUAL_ADAPTER=swiftshader runs headless on Chromium's software
// adapter instead (GPU-less Linux/CI); ANGLE on SwiftShader avoids that loss.
const launchOptions = process.env.SHADER_QUAL_ADAPTER === 'swiftshader'
  ? {
    headless: true,
    args: ['--enable-unsafe-webgpu', '--headless=new', '--enable-features=Vulkan', '--use-vulkan=swiftshader',
      '--use-webgpu-adapter=swiftshader', '--disable-vulkan-surface', '--use-angle=swiftshader'],
  }
  : {
    headless: false,
    args: ['--enable-unsafe-webgpu', '--window-position=2000,2000'],
  }
if (CHROME_EXECUTABLE) launchOptions.executablePath = CHROME_EXECUTABLE

const browser = await chromium.launch(launchOptions)
const report = {
  startedAt: new Date().toISOString(),
  stock: { configs: [], pipelines: null, shaderDiagnostics: null, statusEvents: null },
  patched: { configs: [], pipelines: null, shaderDiagnostics: null, statusEvents: null },
  adapterInfo: null,
  immediateActive: null,
}
async function runPass(label, patched, configs) {
  const page = await browser.newPage({ viewport: { width: 660, height: 520 } })
  page.on('pageerror', error => console.log('pageerror:', error.message))
  page.on('console', message => console.log('console:', message.text()))
  const section = report[label]
  try {
    await page.goto(`http://127.0.0.1:${PORT}/tmp/shader-library-qual/index.html?patched=${patched ? 1 : 0}`)
    await page.waitForFunction(() => document.getElementById('log')?.textContent === 'READY', { timeout: 60000 })
    const meta = await page.evaluate(() => ({
      adapterInfo: window.__qual.adapterInfo,
      pipelines: window.__qual.pipelines,
      shaderDiagnostics: window.__qual.shaderDiagnostics,
      statusEvents: window.__qual.statusEvents,
      immediateActive: window.__qual.immediateActive,
    }))
    report.adapterInfo = meta.adapterInfo
    report.immediateActive = meta.immediateActive
    section.pipelines = meta.pipelines
    section.shaderDiagnostics = meta.shaderDiagnostics
    section.statusEvents = meta.statusEvents
    console.log(`[${label}] adapter:`, JSON.stringify(meta.adapterInfo), 'immediate active:', meta.immediateActive)
    console.log(`[${label}] pipelines:`, JSON.stringify(meta.pipelines))
    console.log(`[${label}] shader diagnostics:`, JSON.stringify(meta.shaderDiagnostics))

    const canvas = page.locator('#view')
    for (const config of configs) {
      const file = `${patched ? '' : 'stock-'}${config.name}.png`
      try {
        const errors = await page.evaluate(cfg => window.__qual.run(cfg), config)
        const path = join(OUT, file)
        const shot = await canvas.screenshot({ path })
        const stats = await page.evaluate(
          dataUrl => window.__qual.imageStats(dataUrl),
          'data:image/png;base64,' + shot.toString('base64'),
        )
        const pass = errors.length === 0 && stats.stddev > 3 && stats.nonBgFraction > 0.02
        section.configs.push({ ...config, stats, errors, screenshot: `output/shader-library-qualification/${file}`, pass })
        console.log(`${pass ? 'PASS' : 'FAIL'} [${label}] ${config.name} stddev=${stats.stddev.toFixed(1)} nonBg=${(stats.nonBgFraction * 100).toFixed(1)}% bg=${stats.background}${errors.length ? ` errors=${JSON.stringify(errors)}` : ''}`)
      } catch (error) {
        section.configs.push({ ...config, error: String(error), pass: false })
        console.log(`FAIL [${label}] ${config.name} ${error}`)
      }
    }
  } finally {
    await page.close()
  }
}
try {
  // Stock pass: the uniformity fix now lives in the committed WGSL sources,
  // so the full matrix must pass unpatched (no ?patched=1).
  await runPass('stock', false, CONFIGS)
  // Patched pass: in-memory uniformity repair, kept as a drift tripwire — if
  // the WGSL regresses, stock fails while patched still passes.
  await runPass('patched', true, CONFIGS)
} finally {
  await browser.close()
  await server.close()
}

const okResults = results => Object.values(results ?? {}).every(result => result === 'ok' || result === 'unsupported' || (Array.isArray(result) && result[0]?.startsWith('unsupported')))
report.finishedAt = new Date().toISOString()
report.pass = report.stock.configs.every(entry => entry.pass)
  && okResults(report.stock.pipelines)
  && okResults(report.stock.shaderDiagnostics)
await writeFile(join(OUT, 'report.json'), JSON.stringify(report, null, 2))
const configRows = section => section.configs.map(entry => entry.error
  ? `| ${entry.name} | — | — | — | FAIL (${entry.error}) | — |`
  : `| ${entry.name} | ${entry.stats.stddev.toFixed(1)} | ${(entry.stats.nonBgFraction * 100).toFixed(1)} | ${entry.errors.length ? entry.errors.join('<br>').replaceAll('|', '\\|') : 'none'} | ${entry.pass ? 'PASS' : 'FAIL'} | ${entry.screenshot.split('/').pop()} |`)
const lines = [
  '# Shader library browser qualification',
  '',
  `- Date: ${report.startedAt}`,
  `- Adapter: ${JSON.stringify(report.adapterInfo)}`,
  `- Immediate variant active: ${report.immediateActive}`,
  `- Overall (stock sources): ${report.pass ? 'PASS' : 'FAIL'}`,
  '',
  '## Stock sources (as committed)',
  '',
  '### Shader module compilation (Tint), shader × variant',
  '',
  '| shader/variant | diagnostics |',
  '| --- | --- |',
  ...Object.entries(report.stock.shaderDiagnostics ?? {}).map(([key, result]) => `| ${key} | ${Array.isArray(result) ? result.join('<br>') : result} |`),
  '',
  '### Pipeline creation',
  '',
  '| pipeline | result |',
  '| --- | --- |',
  ...Object.entries(report.stock.pipelines ?? {}).map(([key, result]) => `| ${key} | ${result} |`),
  '',
  '### Configurations (stock sources)',
  '',
  '| config | stddev | non-bg % | GPU errors | result | screenshot |',
  '| --- | --- | --- | --- | --- | --- |',
  ...configRows(report.stock),
  '',
  '### Status events',
  '',
  ...(report.stock.statusEvents?.length ? report.stock.statusEvents.map(event => `- ${event}`) : ['- none']),
  '',
  '## Patched sources (in-memory uniformity repair)',
  '',
  'The patch rewrites shadowFactor\'s non-uniform bounds early-return as a',
  'clamped select and hoists the shadowFactor call above the section-cap',
  'early return; nothing on disk is modified.',
  '',
  '### Shader module compilation (Tint), shader × variant',
  '',
  '| shader/variant | diagnostics |',
  '| --- | --- |',
  ...Object.entries(report.patched.shaderDiagnostics ?? {}).map(([key, result]) => `| ${key} | ${Array.isArray(result) ? result.join('<br>') : result} |`),
  '',
  '### Pipeline creation',
  '',
  '| pipeline | result |',
  '| --- | --- |',
  ...Object.entries(report.patched.pipelines ?? {}).map(([key, result]) => `| ${key} | ${result} |`),
  '',
  '### Configurations',
  '',
  '| config | stddev | non-bg % | GPU errors | result | screenshot |',
  '| --- | --- | --- | --- | --- | --- |',
  ...configRows(report.patched),
  '',
  '### Status events',
  '',
  ...(report.patched.statusEvents?.length ? report.patched.statusEvents.map(event => `- ${event}`) : ['- none']),
  '',
]
await writeFile(join(OUT, 'report.md'), lines.join('\n'))
console.log(`overall (stock): ${report.pass ? 'PASS' : 'FAIL'} -> ${join(OUT, 'report.md')}`)
process.exit(report.pass ? 0 : 1)
