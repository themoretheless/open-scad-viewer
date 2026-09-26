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

const SHADERS = ['mesh', 'meshPbr', 'meshMatcap', 'meshToon', 'meshUnlit']
const VARIANTS = ['uniform', 'immediate', 'instanced']

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

const adapter = await navigator.gpu.requestAdapter()
const adapterInfo = adapter?.info ? { ...adapter.info } : null

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
renderer.onStatusChange = event => {
  window.__statusEvents.push(JSON.stringify(event, (key, value) => value instanceof Error ? value.message + ' | ' + value.stack : value))
  console.log('status:', window.__statusEvents[window.__statusEvents.length - 1])
}
function frame() {
  return new Promise(resolve => {
    frameResolve = resolve
    renderer.requestRender()
    setTimeout(resolve, 2000)
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

// Pipeline creation matrix: every shading model x every variant. Results are
// 'ok', 'unsupported' (registry rejects the variant), or the error message.
const pipelines = {}
for (const id of SHADERS) {
  for (const variant of VARIANTS) {
    try {
      renderer.getRenderPipeline(id, { variant })
      pipelines[id + '/' + variant] = 'ok'
    } catch (error) {
      pipelines[id + '/' + variant] = /does not support/.test(String(error)) ? 'unsupported' : String(error)
    }
  }
}

window.__qual = {
  adapterInfo,
  pipelines,
  immediateActive: !!renderer.immediateObjectStyle,
  async run(config) {
    renderer.setSection(false, [0, 0, 1], 0)
    renderer.setShadowsEnabled(false)
    renderer.setTheme('default')
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
    await frame()
    await frame()
    return canvasStats()
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

const launchOptions = {
  headless: true,
  args: ['--enable-unsafe-webgpu', '--headless=new'],
}
if (CHROME_EXECUTABLE) launchOptions.executablePath = CHROME_EXECUTABLE

const browser = await chromium.launch(launchOptions)
const report = { startedAt: new Date().toISOString(), configs: [], pipelines: null, adapterInfo: null, immediateActive: null }
try {
  const page = await browser.newPage({ viewport: { width: 660, height: 520 } })
  page.on('pageerror', error => console.log('pageerror:', error.message))
  page.on('console', message => console.log('console:', message.text()))
  await page.goto(`http://127.0.0.1:${PORT}/tmp/shader-library-qual/index.html`)
  await page.waitForFunction(() => document.getElementById('log')?.textContent === 'READY', { timeout: 60000 })
  const meta = await page.evaluate(() => ({
    adapterInfo: window.__qual.adapterInfo,
    pipelines: window.__qual.pipelines,
    immediateActive: window.__qual.immediateActive,
  }))
  report.adapterInfo = meta.adapterInfo
  report.pipelines = meta.pipelines
  report.immediateActive = meta.immediateActive
  console.log('adapter:', JSON.stringify(meta.adapterInfo))
  console.log('immediate variant active:', meta.immediateActive)
  console.log('pipelines:', JSON.stringify(meta.pipelines, null, 1))

  const canvas = page.locator('#view')
  for (const config of CONFIGS) {
    try {
      const stats = await page.evaluate(cfg => window.__qual.run(cfg), config)
      const path = join(OUT, `${config.name}.png`)
      await canvas.screenshot({ path })
      const pass = stats.stddev > 3 && stats.nonBgFraction > 0.02
      report.configs.push({ ...config, stats, screenshot: `output/shader-library-qualification/${config.name}.png`, pass })
      console.log(`${pass ? 'PASS' : 'FAIL'} ${config.name} stddev=${stats.stddev.toFixed(1)} nonBg=${(stats.nonBgFraction * 100).toFixed(1)}%`)
    } catch (error) {
      report.configs.push({ ...config, error: String(error), pass: false })
      console.log(`FAIL ${config.name} ${error}`)
    }
  }
} finally {
  await browser.close()
  await server.close()
}

report.finishedAt = new Date().toISOString()
report.pass = report.configs.every(entry => entry.pass)
  && Object.values(report.pipelines ?? {}).every(result => result === 'ok' || result === 'unsupported')
await writeFile(join(OUT, 'report.json'), JSON.stringify(report, null, 2))
const lines = [
  '# Shader library browser qualification',
  '',
  `- Date: ${report.startedAt}`,
  `- Adapter: ${JSON.stringify(report.adapterInfo)}`,
  `- Immediate variant active: ${report.immediateActive}`,
  `- Overall: ${report.pass ? 'PASS' : 'FAIL'}`,
  '',
  '## Pipeline creation (shading model × variant)',
  '',
  '| pipeline | result |',
  '| --- | --- |',
  ...Object.entries(report.pipelines ?? {}).map(([key, result]) => `| ${key} | ${result} |`),
  '',
  '## Configurations',
  '',
  '| config | stddev | non-bg % | result | screenshot |',
  '| --- | --- | --- | --- | --- |',
  ...report.configs.map(entry => entry.error
    ? `| ${entry.name} | — | — | FAIL (${entry.error}) | — |`
    : `| ${entry.name} | ${entry.stats.stddev.toFixed(1)} | ${(entry.stats.nonBgFraction * 100).toFixed(1)} | ${entry.pass ? 'PASS' : 'FAIL'} | ${entry.name}.png |`),
  '',
]
await writeFile(join(OUT, 'report.md'), lines.join('\n'))
console.log(`overall: ${report.pass ? 'PASS' : 'FAIL'} -> ${join(OUT, 'report.md')}`)
process.exit(report.pass ? 0 : 1)
