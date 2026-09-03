import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { createRequire } from 'node:module'

// This executable is intentionally dependency-free: the parent grants the
// one-shot process read access only to this file and the verified runtime.
const PROTOCOL_VERSION = 1
const RUNTIME_VERSION = '2026.09.01'
const MAX_STDIN_BYTES = 10 * 1024 * 1024
const MAX_SOURCE_BYTES = 1_048_576
const MAX_FILES = 128
const MAX_FILE_BYTES = 4 * 1024 * 1024
const MAX_PROJECT_BYTES = 6 * 1024 * 1024
const MAX_OUTPUT_BYTES = 6 * 1024 * 1024
const MAX_LOG_BYTES = 64 * 1024
const MAX_LOG_ENTRIES = 128
const MAX_LOG_ENTRY_BYTES = 2 * 1024
const FORMATS = new Set(['stl', 'off', 'wrl', '3mf', 'csg', 'dxf', 'svg'])
const EXPERIMENTS = new Set([
  'roof', 'lazy-union', 'vertex-object-renderers-indexing', 'textmetrics',
  'import-function', 'object-function', 'predictible-output', 'vector-swizzle',
  'discretization-by-error', 'ai-features', 'unicode-identifiers',
])
const MIME_TYPES = Object.freeze({
  stl: 'model/stl',
  off: 'model/vnd.off',
  wrl: 'model/vrml',
  '3mf': 'model/3mf',
  csg: 'text/x-openscad-csg',
  dxf: 'image/vnd.dxf',
  svg: 'image/svg+xml',
})

function sha256(value) {
  return createHash('sha256').update(value).digest('hex')
}

function safeMessage(value) {
  let text = value instanceof Error ? value.message : String(value)
  text = text.replace(/[\r\n\u2028\u2029]+/g, ' ').trim() || 'Official OpenSCAD execution failed'
  while (Buffer.byteLength(text) > 4_096) text = text.slice(0, -1)
  return text
}

async function readBoundedStdin() {
  const chunks = []
  let bytes = 0
  for await (const chunk of process.stdin) {
    const data = Buffer.from(chunk)
    bytes += data.byteLength
    if (bytes > MAX_STDIN_BYTES) throw new Error('Official OpenSCAD request exceeds the child input limit')
    chunks.push(data)
  }
  return Buffer.concat(chunks, bytes).toString('utf8')
}

function normalizedPath(value) {
  if (typeof value !== 'string' || value.length === 0 || value.length > 1_024
    || value !== value.normalize('NFC') || value.includes('\\') || value.includes('\0')
    || value.startsWith('/')) return false
  const segments = value.split('/')
  return !segments.some(segment => segment.length === 0 || segment === '.' || segment === '..')
    && value !== 'main.scad' && value !== 'fonts/Basic-Regular.ttf'
    && value !== 'fonts/fonts.conf' && !value.startsWith('__open_scad_result.')
}

function decodeStrictBase64(value) {
  if (typeof value !== 'string' || value.length % 4 !== 0
    || value.length > Math.ceil(MAX_FILE_BYTES / 3) * 4) return null
  const paddingIndex = value.indexOf('=')
  if (paddingIndex >= 0 && (paddingIndex < value.length - 2
    || !/^={1,2}$/.test(value.slice(paddingIndex)))) return null
  const dataEnd = paddingIndex < 0 ? value.length : paddingIndex
  for (let index = 0; index < dataEnd; index += 1) {
    const code = value.charCodeAt(index)
    const allowed = (code >= 0x41 && code <= 0x5a) || (code >= 0x61 && code <= 0x7a)
      || (code >= 0x30 && code <= 0x39) || code === 0x2b || code === 0x2f
    if (!allowed) return null
  }
  const decoded = Buffer.from(value, 'base64')
  return decoded.toString('base64') === value ? decoded : null
}

function validateRequest(value) {
  if (value === null || Array.isArray(value) || typeof value !== 'object'
    || value.protocolVersion !== PROTOCOL_VERSION || !Number.isSafeInteger(value.jobId)
    || value.jobId <= 0 || (value.operation !== 'check' && value.operation !== 'export')
    || typeof value.source !== 'string' || Buffer.byteLength(value.source) > MAX_SOURCE_BYTES
    || typeof value.sourceSha256 !== 'string' || sha256(value.source) !== value.sourceSha256
    || !Array.isArray(value.files) || value.files.length > MAX_FILES
    || (value.operation === 'check' ? value.format !== null : !FORMATS.has(value.format))) {
    throw new Error('Official OpenSCAD child received an invalid request')
  }
  const options = value.options
  if (options === null || Array.isArray(options) || typeof options !== 'object'
    || !Array.isArray(options.experimentalFeatures)
    || !options.experimentalFeatures.every(feature => EXPERIMENTS.has(feature))
    || new Set(options.experimentalFeatures).size !== options.experimentalFeatures.length
    || !Array.isArray(options.defines) || options.defines.length > 64
    || !options.defines.every(define => typeof define === 'string' && !/[\r\n\0]/.test(define)
      && /^[$A-Za-z_][$A-Za-z0-9_]*\s*=/.test(define))
    || (options.time !== null && (typeof options.time !== 'number'
      || !Number.isFinite(options.time) || options.time < 0 || options.time > 1))
    || (options.backend !== 'Manifold' && options.backend !== 'CGAL')
    || typeof options.hardWarnings !== 'boolean'
    || typeof options.checkParameters !== 'boolean'
    || typeof options.checkParameterRanges !== 'boolean') {
    throw new Error('Official OpenSCAD child received invalid execution options')
  }
  let projectBytes = Buffer.byteLength(value.source)
  const paths = new Set()
  for (const file of value.files) {
    if (file === null || Array.isArray(file) || typeof file !== 'object'
      || !normalizedPath(file.path) || paths.has(file.path)) {
      throw new Error('Official OpenSCAD child received an invalid project path')
    }
    const data = decodeStrictBase64(file.dataBase64)
    if (data === null || data.byteLength > MAX_FILE_BYTES || sha256(data) !== file.sha256) {
      throw new Error('Official OpenSCAD child received invalid project data')
    }
    projectBytes += data.byteLength
    if (projectBytes > MAX_PROJECT_BYTES) throw new Error('Official OpenSCAD project exceeds the child byte limit')
    paths.add(file.path)
  }
  return value
}

function createLogs() {
  const stdout = []
  const stderr = []
  let bytes = 0
  let truncated = false
  const append = (target, raw) => {
    let value = String(raw).replace(/[\r\n\u2028\u2029]+/g, ' ').trim()
    if (!value) return
    while (Buffer.byteLength(value) > MAX_LOG_ENTRY_BYTES) value = value.slice(0, -1)
    const valueBytes = Buffer.byteLength(value)
    if (stdout.length + stderr.length >= MAX_LOG_ENTRIES || bytes + valueBytes > MAX_LOG_BYTES) {
      truncated = true
      return
    }
    target.push(value)
    bytes += valueBytes
  }
  return {
    print: value => append(stdout, value),
    printErr: value => append(stderr, value),
    snapshot: () => ({ stdout, stderr, truncated }),
  }
}

function ensureDirectory(FS, path) {
  if (path === '/project') {
    try { FS.mkdir(path) } catch (error) {
      if (!error || error.errno !== 20) throw error
    }
    return
  }
  const segments = path.split('/').filter(Boolean)
  let current = ''
  for (const segment of segments) {
    current += `/${segment}`
    try { FS.mkdir(current) } catch (error) {
      if (!error || error.errno !== 20) throw error
    }
  }
}

function outputExtension(operation, format) {
  return operation === 'check' ? 'csg' : format
}

function cliFormat(operation, format) {
  if (operation === 'check') return 'csg'
  return format === 'stl' ? 'binstl' : format
}

function buildArguments(request, outputPath) {
  const args = [
    `--backend=${request.options.backend}`,
    `--check-parameters=${request.options.checkParameters ? 'true' : 'false'}`,
    `--check-parameter-ranges=${request.options.checkParameterRanges ? 'true' : 'false'}`,
  ]
  if (request.options.hardWarnings) args.push('--hardwarnings')
  for (const feature of request.options.experimentalFeatures) args.push(`--enable=${feature}`)
  for (const define of request.options.defines) args.push('-D', define)
  if (request.options.time !== null) args.push('-D', `$t=${request.options.time}`)
  args.push(`--export-format=${cliFormat(request.operation, request.format)}`, '-o', outputPath, '/project/main.scad')
  return args
}

async function loadRuntime(runtimePath, logs) {
  let initialized = false
  let resolveInitialized
  const initializedPromise = new Promise(resolve => { resolveInitialized = resolve })
  const injected = {
    noInitialRun: true,
    noExitRuntime: true,
    useNodeRawFS: false,
    environment: {
      FONTCONFIG_FILE: '/project/fonts/fonts.conf',
      FONTCONFIG_PATH: '/project/fonts',
      HOME: '/project/home',
    },
    print: logs.print,
    printErr: logs.printErr,
    onRuntimeInitialized() {
      initialized = true
      resolveInitialized()
    },
  }
  globalThis.__OPENSCAD_MODULE__ = injected
  let runtime
  try {
    runtime = createRequire(import.meta.url)(runtimePath)
    if (runtime?.calledRun || runtime?.runtimeInitialized) {
      initialized = true
      resolveInitialized()
    }
    if (!initialized) await initializedPromise
  } finally {
    delete globalThis.__OPENSCAD_MODULE__
  }
  if (!runtime || typeof runtime.callMain !== 'function' || !runtime.FS) {
    throw new Error('Installed OpenSCAD runtime does not expose callMain and MEMFS')
  }
  return runtime
}

const FONTCONFIG = `<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig>
  <dir>/project/fonts</dir>
  <cachedir>/project/home/cache</cachedir>
</fontconfig>
`

async function execute(request, runtimePath, fontPath, expectedFontSha256, logs) {
  const startedAt = performance.now()
  const font = await readFile(fontPath)
  if (font.byteLength === 0 || font.byteLength > 2 * 1024 * 1024
    || sha256(font) !== expectedFontSha256) {
    throw Object.assign(new Error('Installed default font failed child integrity verification'), {
      code: 'E_OPENSCAD_FONT_INTEGRITY',
    })
  }
  const runtime = await loadRuntime(runtimePath, logs)
  const { FS } = runtime
  ensureDirectory(FS, '/project')
  ensureDirectory(FS, '/project/fonts')
  ensureDirectory(FS, '/project/home/cache')
  FS.writeFile('/project/fonts/fonts.conf', FONTCONFIG, { encoding: 'utf8' })
  FS.writeFile('/project/fonts/Basic-Regular.ttf', font)
  FS.writeFile('/project/main.scad', request.source, { encoding: 'utf8' })
  for (const file of request.files) {
    const slash = file.path.lastIndexOf('/')
    if (slash >= 0) ensureDirectory(FS, `/project/${file.path.slice(0, slash)}`)
    FS.writeFile(`/project/${file.path}`, Buffer.from(file.dataBase64, 'base64'))
  }
  FS.chdir('/project')
  const extension = outputExtension(request.operation, request.format)
  const outputPath = `/project/__open_scad_result.${extension}`
  let exitStatus = 0
  try {
    const returned = runtime.callMain(buildArguments(request, outputPath))
    if (Number.isSafeInteger(returned)) exitStatus = returned
  } catch (error) {
    if (error && Number.isSafeInteger(error.status)) exitStatus = error.status
    else throw error
  }
  if (exitStatus !== 0) throw Object.assign(new Error(`OpenSCAD exited with status ${exitStatus}`), { code: 'E_OPENSCAD_COMPILE' })
  const warning = logs.snapshot().stderr.find(line => /^(?:ERROR|WARNING):/i.test(line))
  if (warning && request.options.hardWarnings) {
    throw Object.assign(new Error(warning), { code: 'E_OPENSCAD_COMPILE' })
  }
  let stat
  try { stat = FS.stat(outputPath) } catch {
    throw Object.assign(new Error('OpenSCAD did not produce the requested output'), { code: 'E_OPENSCAD_NO_OUTPUT' })
  }
  if (!Number.isSafeInteger(stat.size) || stat.size < 0 || stat.size > MAX_OUTPUT_BYTES) {
    throw Object.assign(new Error(`OpenSCAD output exceeds ${MAX_OUTPUT_BYTES} bytes`), { code: 'E_OPENSCAD_OUTPUT_LIMIT' })
  }
  const output = Buffer.from(FS.readFile(outputPath))
  if (output.byteLength !== stat.size || output.byteLength > MAX_OUTPUT_BYTES) {
    throw Object.assign(new Error('OpenSCAD output violated its declared bounded size'), { code: 'E_OPENSCAD_OUTPUT_LIMIT' })
  }
  const format = request.operation === 'check' ? 'csg' : request.format
  return {
    protocolVersion: PROTOCOL_VERSION,
    jobId: request.jobId,
    operation: request.operation,
    sourceSha256: request.sourceSha256,
    runtimeVersion: RUNTIME_VERSION,
    durationMs: Math.max(0, performance.now() - startedAt),
    logs: logs.snapshot(),
    status: 'succeeded',
    output: {
      format,
      mimeType: MIME_TYPES[format],
      byteLength: output.byteLength,
      sha256: sha256(output),
      dataBase64: request.operation === 'check' ? null : output.toString('base64'),
    },
  }
}

let request = null
const logs = createLogs()
const processStartedAt = performance.now()
try {
  const runtimePath = process.argv[2]
  const fontPath = process.argv[3]
  const expectedFontSha256 = process.argv[4]
  if (typeof runtimePath !== 'string' || runtimePath.length === 0) throw new Error('Official OpenSCAD runtime path is missing')
  if (typeof fontPath !== 'string' || fontPath.length === 0
    || typeof expectedFontSha256 !== 'string' || !/^[0-9a-f]{64}$/.test(expectedFontSha256)) {
    throw new Error('Official OpenSCAD default font metadata is missing')
  }
  request = validateRequest(JSON.parse(await readBoundedStdin()))
  process.stdout.write(JSON.stringify(await execute(
    request,
    runtimePath,
    fontPath,
    expectedFontSha256,
    logs,
  )))
} catch (error) {
  const terminal = {
    protocolVersion: PROTOCOL_VERSION,
    jobId: request?.jobId ?? 1,
    operation: request?.operation ?? 'check',
    sourceSha256: request?.sourceSha256 ?? '0'.repeat(64),
    runtimeVersion: RUNTIME_VERSION,
    durationMs: Math.max(0, performance.now() - processStartedAt),
    logs: logs.snapshot(),
    status: 'failed',
    error: {
      code: typeof error?.code === 'string' && /^[A-Z][A-Z0-9_]{0,63}$/.test(error.code)
        ? error.code
        : 'E_OPENSCAD_RUNTIME',
      name: typeof error?.name === 'string' && error.name ? error.name.slice(0, 128) : 'Error',
      message: safeMessage(error),
    },
  }
  process.stdout.write(JSON.stringify(terminal))
  process.exitCode = 1
}
