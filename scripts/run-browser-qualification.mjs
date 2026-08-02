#!/usr/bin/env node

import { createHash } from 'node:crypto'
import { constants as fsConstants, createReadStream } from 'node:fs'
import { access, readFile, readdir, stat } from 'node:fs/promises'
import { isAbsolute, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  QUALIFICATION_PLAYWRIGHT_PACKAGE,
  isolatedBrowserProvisionCommand,
  loadQualificationPlaywrightPackage,
} from './qualificationPlaywrightPackage.mjs'

export const BROWSER_QUALIFICATION_SCHEMA_VERSION = 'browser-qualification-evidence-v1'
export const EXPECTED_PLAYWRIGHT_VERSION = QUALIFICATION_PLAYWRIGHT_PACKAGE.playwrightVersion
export const REQUIRED_PLAYWRIGHT_DEPENDENCY = QUALIFICATION_PLAYWRIGHT_PACKAGE.playwrightDependency

const ALLOWED_BROWSERS = new Set(['chromium', 'firefox', 'webkit'])
const DEFAULT_CLEAN_RUNS = 1
const DEFAULT_ENGINE_TIMEOUT_MS = 120_000
const MIN_ENGINE_TIMEOUT_MS = 1_000
const MAX_ENGINE_TIMEOUT_MS = 600_000
const MAX_CLEAN_RUNS = 1
const CLEANUP_TIMEOUT_MS = 5_000
const LATE_SETTLEMENT_DRAIN_TIMEOUT_MS = 5_000
const MAX_RESULT_BYTES = 64 * 1024
const MAX_BUILD_FILES = 64
const MAX_BUILD_OUTPUT_BYTES = 8 * 1024 * 1024
const MAX_LOG_ENTRIES = 64
const MAX_LOG_BYTES = 32 * 1024
const MAX_LOG_MESSAGE_BYTES = 2 * 1024
const MAX_WORKER_OBSERVATIONS = 32
const RESULT_SELECTOR = '[data-testid="qualification-result"]'
const QUALIFICATION_ENTRY_PATH = 'tests/fixtures/browser-qualification.html'

export const EXPECTED_BROWSER_QUALIFICATION_CHECK_NAMES = Object.freeze([
  'default Vite Worker exact 256 entity boundary',
  'default Vite Worker realm terminated after success',
  'default Vite Worker rejects 257 identity before publication',
  'default lane recovers in fresh realm',
  'non-cooperative Worker entered execution before deadline',
  'browser main thread remained responsive',
  'wedged browser realm termination requested before settlement',
  'post-quarantine request uses fresh browser realm',
  'controlled entityId length 256 crosses actual Vite Worker boundary',
  'entityId exact case leaves instanceId below its boundary',
  'entityId exact case leaves operationId below its boundary',
  'controlled entityId length 257 is bounded before success publication',
  'entityId refusal recovers in a fresh Vite Worker realm',
  'controlled instanceId length 256 crosses actual Vite Worker boundary',
  'instanceId exact case leaves entityId below its boundary',
  'instanceId exact case leaves operationId below its boundary',
  'controlled instanceId length 257 is bounded before success publication',
  'instanceId refusal recovers in a fresh Vite Worker realm',
  'controlled operationId length 256 crosses actual Vite Worker boundary',
  'operationId exact case leaves entityId below its boundary',
  'operationId exact case leaves instanceId below its boundary',
  'controlled operationId length 257 is bounded before success publication',
  'operationId refusal recovers in a fresh Vite Worker realm',
  'all controlled identity realms terminated',
])

const repositoryRoot = fileURLToPath(new URL('../', import.meta.url))
const qualificationConfigPath = join(repositoryRoot, 'vite.qualification.config.ts')
const qualificationOutputRoot = join(repositoryRoot, 'tmp', 'browser-qualification-dist')

export class BrowserQualificationRunnerError extends Error {
  constructor(code, phase, message, details = {}, options = undefined) {
    super(message, options)
    this.name = 'BrowserQualificationRunnerError'
    this.code = code
    this.phase = phase
    this.details = details
  }
}

function utf8Bytes(value) {
  return Buffer.byteLength(value, 'utf8')
}

function truncateUtf8(value, maximumBytes) {
  const text = String(value)
  if (utf8Bytes(text) <= maximumBytes) return text
  let low = 0
  let high = text.length
  while (low < high) {
    const middle = Math.ceil((low + high) / 2)
    if (utf8Bytes(text.slice(0, middle)) <= maximumBytes - 3) low = middle
    else high = middle - 1
  }
  return text.slice(0, low) + '...'
}

function boundedValue(value, depth = 0) {
  if (value === null || typeof value === 'boolean' || typeof value === 'number') return value
  if (typeof value === 'string') return truncateUtf8(value, MAX_LOG_MESSAGE_BYTES)
  if (depth >= 3) return '[depth-limited]'
  if (Array.isArray(value)) return value.slice(0, 32).map((item) => boundedValue(item, depth + 1))
  if (typeof value === 'object') {
    const output = {}
    for (const key of Object.keys(value).sort().slice(0, 32)) {
      output[truncateUtf8(key, 128)] = boundedValue(value[key], depth + 1)
    }
    return output
  }
  return truncateUtf8(String(value), MAX_LOG_MESSAGE_BYTES)
}

function hasExactOwnKeys(value, expectedKeys) {
  const actualKeys = Object.keys(value).sort()
  const sortedExpected = [...expectedKeys].sort()
  return actualKeys.length === sortedExpected.length
    && actualKeys.every((key, index) => key === sortedExpected[index])
}

export class BoundedQualificationLog {
  constructor(maximumEntries = MAX_LOG_ENTRIES, maximumBytes = MAX_LOG_BYTES) {
    this.maximumEntries = maximumEntries
    this.maximumBytes = maximumBytes
    this.entries = []
    this.retainedBytes = 0
    this.droppedEntries = 0
  }

  add(level, source, message, details = undefined) {
    const entry = {
      level: truncateUtf8(level, 32),
      source: truncateUtf8(source, 64),
      message: truncateUtf8(message, MAX_LOG_MESSAGE_BYTES),
    }
    if (details !== undefined) entry.details = boundedValue(details)
    const bytes = utf8Bytes(JSON.stringify(entry))
    if (this.entries.length >= this.maximumEntries || this.retainedBytes + bytes > this.maximumBytes) {
      this.droppedEntries += 1
      return
    }
    this.entries.push(entry)
    this.retainedBytes += bytes
  }

  snapshot() {
    return {
      entries: this.entries.map((entry) => ({ ...entry })),
      retainedBytes: this.retainedBytes,
      droppedEntries: this.droppedEntries,
      limits: {
        maximumEntries: this.maximumEntries,
        maximumBytes: this.maximumBytes,
        maximumMessageBytes: MAX_LOG_MESSAGE_BYTES,
      },
    }
  }
}

class EngineDeadline {
  constructor(timeoutMs) {
    this.timeoutMs = timeoutMs
    this.startedAt = performance.now()
    this.expiresAt = this.startedAt + timeoutMs
  }

  remainingMs() {
    return Math.max(0, Math.ceil(this.expiresAt - performance.now()))
  }

  async run(phase, operation) {
    const remaining = this.remainingMs()
    if (remaining <= 0) throw engineTimeout(phase, this.timeoutMs)
    let timer
    try {
      return await Promise.race([
        Promise.resolve().then(operation),
        new Promise((_, reject) => {
          timer = setTimeout(() => reject(engineTimeout(phase, this.timeoutMs)), remaining)
        }),
      ])
    } finally {
      if (timer !== undefined) clearTimeout(timer)
    }
  }
}

function engineTimeout(phase, timeoutMs) {
  return new BrowserQualificationRunnerError(
    'E_BROWSER_ENGINE_TIMEOUT',
    phase,
    'Browser qualification engine exceeded its bounded deadline',
    { timeoutMs },
  )
}

function exactLoopbackOrigin(origin) {
  try {
    const parsed = new URL(origin)
    return parsed.origin === origin
      && parsed.protocol === 'http:'
      && parsed.hostname === '127.0.0.1'
      && parsed.port.length > 0
      && parsed.username.length === 0
      && parsed.password.length === 0
  } catch {
    return false
  }
}

/**
 * Qualification pages may read only their exact ephemeral loopback origin.
 * Same-origin blob URLs and data URLs are browser-local transports used by
 * emitted module assets; every other scheme/origin is denied.
 */
export function isQualificationNetworkUrlAllowed(rawUrl, exactOrigin) {
  if (!exactLoopbackOrigin(exactOrigin)) return false
  try {
    const parsed = new URL(rawUrl)
    if (parsed.protocol === 'http:') {
      return parsed.origin === exactOrigin
        && parsed.username.length === 0
        && parsed.password.length === 0
    }
    if (parsed.protocol === 'blob:') return parsed.origin === exactOrigin
    return parsed.protocol === 'data:'
  } catch {
    return false
  }
}

/** Vite HMR may use only the WS counterpart of the exact HTTP loopback origin. */
export function isQualificationWebSocketUrlAllowed(rawUrl, exactOrigin) {
  if (!exactLoopbackOrigin(exactOrigin)) return false
  try {
    const expected = new URL(exactOrigin)
    const parsed = new URL(rawUrl)
    return parsed.protocol === 'ws:'
      && parsed.hostname === expected.hostname
      && parsed.port === expected.port
      && parsed.username.length === 0
      && parsed.password.length === 0
  } catch {
    return false
  }
}

function parseInteger(name, raw, minimum, maximum) {
  if (!/^[0-9]+$/.test(raw ?? '')) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_QUALIFICATION_USAGE',
      'arguments',
      name + ' must be an integer',
      { option: name },
    )
  }
  const value = Number(raw)
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_QUALIFICATION_USAGE',
      'arguments',
      name + ' is outside its admitted range',
      { option: name, minimum, maximum },
    )
  }
  return value
}

export function parseBrowserQualificationArguments(argv) {
  const options = {
    browser: undefined,
    cleanRuns: DEFAULT_CLEAN_RUNS,
    engineTimeoutMs: DEFAULT_ENGINE_TIMEOUT_MS,
    help: false,
  }
  const seen = new Set()
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index]
    if (argument === '--help' || argument === '-h') {
      options.help = true
      continue
    }
    const separatorIndex = argument.indexOf('=')
    const name = separatorIndex === -1 ? argument : argument.slice(0, separatorIndex)
    let value = separatorIndex === -1 ? undefined : argument.slice(separatorIndex + 1)
    if (!['--browser', '--clean-runs', '--timeout-ms'].includes(name)) {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_QUALIFICATION_USAGE',
        'arguments',
        'Unknown browser qualification option',
        { option: argument },
      )
    }
    if (seen.has(name)) {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_QUALIFICATION_USAGE',
        'arguments',
        'Browser qualification option was repeated',
        { option: name },
      )
    }
    seen.add(name)
    if (value === undefined) {
      index += 1
      value = argv[index]
    }
    if (value === undefined || value.startsWith('--')) {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_QUALIFICATION_USAGE',
        'arguments',
        'Browser qualification option is missing its value',
        { option: name },
      )
    }
    if (name === '--browser') options.browser = value
    if (name === '--clean-runs') options.cleanRuns = parseInteger(name, value, 1, MAX_CLEAN_RUNS)
    if (name === '--timeout-ms') {
      options.engineTimeoutMs = parseInteger(
        name,
        value,
        MIN_ENGINE_TIMEOUT_MS,
        MAX_ENGINE_TIMEOUT_MS,
      )
    }
  }
  if (!options.help && !ALLOWED_BROWSERS.has(options.browser)) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_QUALIFICATION_USAGE',
      'arguments',
      'Exactly one supported browser engine is required',
      { allowedBrowsers: [...ALLOWED_BROWSERS] },
    )
  }
  return options
}

function usage() {
  return [
    'Usage: node scripts/run-browser-qualification.mjs --browser <chromium|firefox|webkit> [options]',
    '',
    'Options:',
    '  --clean-runs 1            One clean-run fragment in this fresh process (default: 1)',
    '  --timeout-ms <ms>         Total deadline for one browser-engine invocation (default: 120000)',
    '  --help                    Show this help',
    '',
    'Run required repetitions through an external orchestrator using a fresh process each time.',
    'Requires exactly ' + REQUIRED_PLAYWRIGHT_DEPENDENCY + ' and its Playwright-managed browser binary.',
  ].join('\n')
}

async function sha256File(path) {
  const hash = createHash('sha256')
  for await (const chunk of createReadStream(path)) hash.update(chunk)
  return hash.digest('hex')
}

export async function preflightPlaywright(browserName, dependencies = {}) {
  const loadPackage = dependencies.loadQualificationPackage ?? loadQualificationPlaywrightPackage
  const accessExecutable = dependencies.accessExecutable ?? ((path) => access(
    path,
    process.platform === 'win32' ? fsConstants.F_OK : fsConstants.F_OK | fsConstants.X_OK,
  ))
  const hashExecutable = dependencies.hashExecutable ?? sha256File

  let loadedPackage
  try {
    loadedPackage = await loadPackage()
  } catch (cause) {
    throw new BrowserQualificationRunnerError(
      'E_PLAYWRIGHT_PREFLIGHT_MISSING',
      'preflight',
      'The exact Playwright harness dependency is not available',
      {
        requiredDependency: REQUIRED_PLAYWRIGHT_DEPENDENCY,
        qualificationPackage: QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot,
        installCommand: QUALIFICATION_PLAYWRIGHT_PACKAGE.installCommand,
        causeCode: typeof cause?.code === 'string' ? cause.code : null,
      },
      { cause },
    )
  }
  const playwright = loadedPackage.playwright
  const version = loadedPackage.manifest?.version
  if (version !== EXPECTED_PLAYWRIGHT_VERSION) {
    throw new BrowserQualificationRunnerError(
      'E_PLAYWRIGHT_PREFLIGHT_VERSION',
      'preflight',
      'The installed Playwright version does not match the frozen runner version',
      {
        requiredDependency: REQUIRED_PLAYWRIGHT_DEPENDENCY,
        installedVersion: version,
      },
    )
  }
  const browserType = playwright[browserName]
  if (browserType === undefined || typeof browserType.launch !== 'function'
      || typeof browserType.executablePath !== 'function' || browserType.name() !== browserName) {
    throw new BrowserQualificationRunnerError(
      'E_PLAYWRIGHT_PREFLIGHT_ENGINE',
      'preflight',
      'Playwright did not expose the exact requested browser engine',
      { browser: browserName },
    )
  }
  const executablePath = browserType.executablePath()
  if (!isAbsolute(executablePath)) {
    throw new BrowserQualificationRunnerError(
      'E_PLAYWRIGHT_PREFLIGHT_BINARY',
      'preflight',
      'Playwright returned a non-absolute managed browser executable path',
      { browser: browserName, executablePath },
    )
  }
  try {
    await accessExecutable(executablePath)
  } catch (cause) {
    throw new BrowserQualificationRunnerError(
      'E_PLAYWRIGHT_BROWSER_BINARY_MISSING',
      'preflight',
      'The Playwright-managed browser executable is not installed or executable',
      {
        browser: browserName,
        executablePath,
        requiredDependency: REQUIRED_PLAYWRIGHT_DEPENDENCY,
        provisionCommand: isolatedBrowserProvisionCommand(browserName),
        provisionEnvironment: QUALIFICATION_PLAYWRIGHT_PACKAGE.browserProvisionEnvironment,
      },
      { cause },
    )
  }
  let executableSha256
  try {
    executableSha256 = await hashExecutable(executablePath)
  } catch (cause) {
    throw new BrowserQualificationRunnerError(
      'E_PLAYWRIGHT_PREFLIGHT_BINARY_HASH',
      'preflight',
      'The Playwright-managed browser executable could not be hashed',
      { browser: browserName, executablePath },
      { cause },
    )
  }
  return {
    browserType,
    metadata: {
      version,
      requiredDependency: REQUIRED_PLAYWRIGHT_DEPENDENCY,
      browser: browserName,
      managedBinary: true,
      executablePath,
      executableSha256,
      qualificationPackage: loadedPackage.packageMetadata,
    },
  }
}

function isInside(parent, candidate) {
  const path = relative(resolve(parent), resolve(candidate))
  return path.length > 0 && path !== '..' && !path.startsWith('..' + sep) && !isAbsolute(path)
}

function qualificationPaths() {
  const temporaryRoot = join(repositoryRoot, 'tmp')
  if (!isInside(temporaryRoot, qualificationOutputRoot)
      || resolve(qualificationOutputRoot) === resolve(join(repositoryRoot, 'dist'))) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_BUILD_ISOLATION',
      'build',
      'Qualification output is not isolated under the repository temporary root',
    )
  }
  return {
    root: repositoryRoot,
    configFile: qualificationConfigPath,
    outDir: qualificationOutputRoot,
    entryPath: QUALIFICATION_ENTRY_PATH,
  }
}

function createViteLogger(vite, logs) {
  const logger = vite.createLogger('silent', { allowClearScreen: false })
  logger.info = (message) => logs.add('info', 'vite', message)
  logger.warn = (message) => logs.add('warn', 'vite', message)
  logger.warnOnce = (message) => logs.add('warn', 'vite', message)
  logger.error = (message, options) => logs.add('error', 'vite', message, {
    timestamp: options?.timestamp,
  })
  logger.clearScreen = () => {}
  return logger
}

async function collectArtifactPaths(directory, prefix = '') {
  const entries = await readdir(directory, { withFileTypes: true })
  const paths = []
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    const childPrefix = prefix.length === 0 ? entry.name : prefix + '/' + entry.name
    const childPath = join(directory, entry.name)
    if (entry.isSymbolicLink()) {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_BUILD_ARTIFACT',
        'build',
        'Qualification build emitted a symbolic link',
        { path: childPrefix },
      )
    }
    if (entry.isDirectory()) paths.push(...await collectArtifactPaths(childPath, childPrefix))
    else if (entry.isFile()) paths.push(childPrefix)
    if (paths.length > MAX_BUILD_FILES) {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_BUILD_ARTIFACT_LIMIT',
        'build',
        'Qualification build emitted too many artifacts for bounded evidence',
        { maximumFiles: MAX_BUILD_FILES },
      )
    }
  }
  return paths
}

export function validateBuildArtifactByteBudget(artifacts) {
  let totalBytes = 0
  for (const artifact of artifacts) {
    if (!Number.isSafeInteger(artifact?.bytes) || artifact.bytes < 0) {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_BUILD_ARTIFACT',
        'build',
        'Qualification build artifact has an invalid byte length',
      )
    }
    totalBytes += artifact.bytes
    if (!Number.isSafeInteger(totalBytes) || totalBytes > MAX_BUILD_OUTPUT_BYTES) {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_BUILD_ARTIFACT_BYTES_LIMIT',
        'build',
        'Qualification build exceeded the aggregate artifact byte budget',
        { maximumBytes: MAX_BUILD_OUTPUT_BYTES, observedBytes: totalBytes },
      )
    }
  }
  return totalBytes
}

async function inspectQualificationBuild(paths) {
  const artifactPaths = await collectArtifactPaths(paths.outDir)
  const artifacts = []
  for (const artifactPath of artifactPaths) {
    const absolutePath = join(paths.outDir, artifactPath)
    const metadata = await stat(absolutePath)
    const artifactMetadata = {
      path: artifactPath,
      bytes: metadata.size,
    }
    validateBuildArtifactByteBudget([...artifacts, artifactMetadata])
    artifacts.push({
      ...artifactMetadata,
      sha256: await sha256File(absolutePath),
    })
  }
  const entry = artifacts.find((artifact) => artifact.path === paths.entryPath)
  if (entry === undefined || entry.bytes <= 0 || entry.bytes > MAX_RESULT_BYTES) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_BUILD_ENTRY',
      'build',
      'Qualification build did not emit the bounded test-only HTML entry',
      { entryPath: paths.entryPath },
    )
  }
  const entryHtml = await readFile(join(paths.outDir, paths.entryPath), 'utf8')
  if (!entryHtml.includes('data-testid="qualification-result"')) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_BUILD_ENTRY',
      'build',
      'Qualification build entry is missing the exact result marker',
      { selector: RESULT_SELECTOR },
    )
  }
  const workerArtifacts = artifacts.filter((artifact) => /(^|\/)assets\/[^/]*\.worker-[^/]+\.js$/.test(artifact.path))
  if (workerArtifacts.length === 0) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_BUILD_WORKER',
      'build',
      'Qualification build did not emit a module Worker artifact',
    )
  }
  return {
    configPath: relative(repositoryRoot, paths.configFile).replaceAll(sep, '/'),
    outputPath: relative(repositoryRoot, paths.outDir).replaceAll(sep, '/'),
    entryPath: paths.entryPath,
    artifacts,
    totalBytes: validateBuildArtifactByteBudget(artifacts),
    workerArtifacts: workerArtifacts.map((artifact) => artifact.path),
  }
}

export async function buildQualificationBundle(options = {}) {
  const vite = options.vite ?? await import('vite')
  const deadline = options.deadline ?? new EngineDeadline(DEFAULT_ENGINE_TIMEOUT_MS)
  const logs = options.logs ?? new BoundedQualificationLog()
  const paths = qualificationPaths()
  try {
    await deadline.run('vite-build', () => vite.build({
      root: paths.root,
      configFile: paths.configFile,
      logLevel: 'silent',
      clearScreen: false,
      customLogger: createViteLogger(vite, logs),
      build: {
        outDir: paths.outDir,
        emptyOutDir: true,
      },
    }))
    return await deadline.run('build-inspection', () => inspectQualificationBuild(paths))
  } catch (cause) {
    if (cause instanceof BrowserQualificationRunnerError) throw cause
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_BUILD_FAILED',
      'build',
      'The isolated browser qualification bundle could not be built',
      { cause: truncateUtf8(cause instanceof Error ? cause.message : String(cause), MAX_LOG_MESSAGE_BYTES) },
      { cause },
    )
  }
}

export async function startQualificationPreview(options = {}) {
  const vite = options.vite ?? await import('vite')
  const deadline = options.deadline ?? new EngineDeadline(DEFAULT_ENGINE_TIMEOUT_MS)
  const logs = options.logs ?? new BoundedQualificationLog()
  const ownsQuarantine = options.quarantine === undefined
  const quarantine = options.quarantine ?? new BrowserLateSettlementQuarantine(logs)
  const paths = qualificationPaths()
  let server
  try {
    server = await acquireBrowserResource(deadline, 'vite-preview', () => vite.preview({
      root: paths.root,
      configFile: paths.configFile,
      logLevel: 'silent',
      clearScreen: false,
      customLogger: createViteLogger(vite, logs),
      build: { outDir: paths.outDir },
      preview: {
        host: '127.0.0.1',
        port: 0,
        strictPort: true,
        open: false,
      },
    }), quarantine, (lateServer) => lateServer.close())
    const address = server.httpServer.address()
    if (address === null || typeof address === 'string' || address.port <= 0
        || address.address !== '127.0.0.1') {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_PREVIEW_ADDRESS',
        'serve',
        'Vite preview did not bind an isolated loopback port',
        {
          observedAddress: address === null || typeof address === 'string'
            ? address
            : address.address,
        },
      )
    }
    const origin = 'http://127.0.0.1:' + address.port
    return {
      server,
      origin,
      entryUrl: origin + '/' + QUALIFICATION_ENTRY_PATH,
    }
  } catch (cause) {
    const primary = cause instanceof BrowserQualificationRunnerError
      ? cause
      : new BrowserQualificationRunnerError(
        'E_BROWSER_PREVIEW_FAILED',
        'serve',
        'The isolated Vite preview server could not start',
        { cause: truncateUtf8(cause instanceof Error ? cause.message : String(cause), MAX_LOG_MESSAGE_BYTES) },
        { cause },
      )
    const cleanupErrors = []
    if (server !== undefined) {
      const cleanupError = await cleanupResource('vite-preview', () => server.close(), logs)
      if (cleanupError !== undefined) cleanupErrors.push(cleanupError)
    }
    if (ownsQuarantine) cleanupErrors.push(...await quarantine.drain())
    throw combinePrimaryFirst(primary, cleanupErrors, 'Vite preview acquisition and cleanup failed')
  }
}

export function validateQualificationPayload(status, payload) {
  const expectedPayloadKeys = status === 'passed'
    ? ['status', 'checks']
    : ['status', 'checks', 'error']
  if (payload === null || typeof payload !== 'object' || Array.isArray(payload)
      || !hasExactOwnKeys(payload, expectedPayloadKeys)
      || (payload.status !== 'passed' && payload.status !== 'failed')
      || payload.status !== status || !Array.isArray(payload.checks)) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_RESULT_INVALID',
      'result',
      'Qualification result marker did not contain the exact JSON result contract',
    )
  }
  for (const check of payload.checks) {
    if (check === null || typeof check !== 'object' || Array.isArray(check)
        || !hasExactOwnKeys(check, ['name', 'passed'])
        || typeof check.name !== 'string' || typeof check.passed !== 'boolean') {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_RESULT_INVALID',
        'result',
        'Qualification result contained an invalid check record',
      )
    }
  }
  if (status === 'failed' || payload.checks.some((check) => !check.passed)) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_QUALIFICATION_FAILED',
      'result',
      'The browser qualification fixture reported failure',
      { fixtureError: boundedValue(payload.error) },
    )
  }
  const actualNames = payload.checks.map((check) => check.name)
  const uniqueNames = new Set(actualNames)
  const exactCoverage = actualNames.length === EXPECTED_BROWSER_QUALIFICATION_CHECK_NAMES.length
    && actualNames.every((name, index) => name === EXPECTED_BROWSER_QUALIFICATION_CHECK_NAMES[index])
    && uniqueNames.size === actualNames.length
  if (!exactCoverage) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_RESULT_COVERAGE',
      'result',
      'Qualification result did not match the exact frozen check-name coverage and order',
      {
        expectedCount: EXPECTED_BROWSER_QUALIFICATION_CHECK_NAMES.length,
        actualCount: actualNames.length,
        uniqueCount: uniqueNames.size,
        expectedNames: EXPECTED_BROWSER_QUALIFICATION_CHECK_NAMES,
        actualNames,
      },
    )
  }
  return payload
}

export async function readExactQualificationResult(page, deadline) {
  const locator = page.locator(RESULT_SELECTOR)
  await deadline.run('qualification-result-marker', () => locator.waitFor({
    state: 'attached',
    timeout: Math.max(1, deadline.remainingMs()),
  }))
  const initialCount = await deadline.run('qualification-result-count', () => locator.count())
  if (initialCount !== 1) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_RESULT_MARKER_COUNT',
      'result',
      'Qualification page must expose exactly one result marker',
      { selector: RESULT_SELECTOR, count: initialCount },
    )
  }
  await deadline.run('qualification-result-terminal', () => page.waitForFunction(
    (selector) => {
      const elements = document.querySelectorAll(selector)
      if (elements.length !== 1) return false
      const status = elements[0].getAttribute('data-status')
      return status === 'passed' || status === 'failed'
    },
    RESULT_SELECTOR,
    { timeout: Math.max(1, deadline.remainingMs()) },
  ))
  const finalCount = await deadline.run('qualification-result-final-count', () => locator.count())
  if (finalCount !== 1) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_RESULT_MARKER_COUNT',
      'result',
      'Qualification page changed the exact result marker cardinality',
      { selector: RESULT_SELECTOR, count: finalCount },
    )
  }
  const captured = await deadline.run('qualification-result-read', () => locator.evaluate(
    (element, maximumBytes) => {
      const text = element.textContent ?? ''
      const byteLength = new TextEncoder().encode(text).byteLength
      return {
        status: element.getAttribute('data-status'),
        byteLength,
        text: byteLength <= maximumBytes ? text : null,
      }
    },
    MAX_RESULT_BYTES,
  ))
  if (captured.text === null) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_RESULT_TOO_LARGE',
      'result',
      'Qualification result exceeded the bounded evidence limit',
      { maximumBytes: MAX_RESULT_BYTES, actualBytes: captured.byteLength },
    )
  }
  let payload
  try {
    payload = JSON.parse(captured.text)
  } catch (cause) {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_RESULT_INVALID_JSON',
      'result',
      'Qualification result marker did not contain valid JSON',
      {},
      { cause },
    )
  }
  return validateQualificationPayload(captured.status, payload)
}

function relativeArtifactFromUrl(url, origin) {
  try {
    const parsed = new URL(url)
    if (parsed.origin !== origin) return undefined
    return decodeURIComponent(parsed.pathname.slice(1))
  } catch {
    return undefined
  }
}

async function cleanupResource(label, operation, logs) {
  let timer
  try {
    await Promise.race([
      Promise.resolve().then(operation),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(label + ' cleanup timed out')), CLEANUP_TIMEOUT_MS)
      }),
    ])
    return undefined
  } catch (error) {
    logs.add('error', 'cleanup', label + ' cleanup failed', {
      message: error instanceof Error ? error.message : String(error),
    })
    return new BrowserQualificationRunnerError(
      'E_BROWSER_CLEANUP_FAILED',
      'cleanup',
      'Browser qualification resource cleanup failed',
      {
        resource: label,
        cleanupDeadlineMs: CLEANUP_TIMEOUT_MS,
        quarantined: true,
        hardKillJoinClaim: false,
        externalSupervisorRequiredForHardKillAndJoin: true,
      },
      { cause: error },
    )
  } finally {
    if (timer !== undefined) clearTimeout(timer)
  }
}

function combinePrimaryFirst(primary, cleanupErrors, message) {
  if (primary === undefined && cleanupErrors.length === 0) return undefined
  if (primary === undefined && cleanupErrors.length === 1) return cleanupErrors[0]
  if (primary !== undefined && cleanupErrors.length === 0) return primary
  return new AggregateError(
    primary === undefined ? cleanupErrors : [primary, ...cleanupErrors],
    message,
  )
}

export class BrowserLateSettlementQuarantine {
  constructor(logs = new BoundedQualificationLog()) {
    this.logs = logs
    this.pending = []
    this.labels = []
  }

  trackAcquisition(label, acquisition, cleanupLate) {
    this.labels.push(label)
    const pending = acquisition.then(async (resource) => {
      const cleanupError = await cleanupResource('late-' + label, () => cleanupLate(resource), this.logs)
      if (cleanupError !== undefined) throw cleanupError
    }, () => undefined)
    this.pending.push(pending)
  }

  async drain(timeoutMs = LATE_SETTLEMENT_DRAIN_TIMEOUT_MS) {
    if (this.pending.length === 0) return []
    let timer
    try {
      const settled = await Promise.race([
        Promise.allSettled(this.pending),
        new Promise((resolvePromise) => {
          timer = setTimeout(() => resolvePromise(null), timeoutMs)
        }),
      ])
      if (settled === null) {
        return [new BrowserQualificationRunnerError(
          'E_BROWSER_LATE_ACQUISITION_UNJOINED',
          'cleanup',
          'A timed-out browser resource acquisition did not settle within quarantine drain',
          {
            resources: [...this.labels],
            cleanupDeadlineMs: timeoutMs,
            quarantined: true,
            hardKillJoinClaim: false,
            externalSupervisorRequiredForHardKillAndJoin: true,
          },
        )]
      }
      return settled
        .filter((result) => result.status === 'rejected')
        .map((result) => result.reason)
    } finally {
      if (timer !== undefined) clearTimeout(timer)
    }
  }
}

export async function acquireBrowserResource(
  deadline,
  phase,
  operation,
  quarantine,
  cleanupLate,
) {
  const acquisition = Promise.resolve().then(operation)
  try {
    return await deadline.run(phase, () => acquisition)
  } catch (error) {
    if (error instanceof BrowserQualificationRunnerError
        && error.code === 'E_BROWSER_ENGINE_TIMEOUT') {
      quarantine.trackAcquisition(phase, acquisition, cleanupLate)
      error.details = {
        ...error.details,
        quarantinedResource: phase,
        lateSettlementCleanupArmed: true,
        hardKillJoinClaim: false,
        externalSupervisorRequiredForHardKillAndJoin: true,
      }
    }
    throw error
  }
}

export async function installQualificationNetworkIsolation(context, exactOrigin) {
  if (!exactLoopbackOrigin(exactOrigin)
      || typeof context?.route !== 'function'
      || typeof context?.routeWebSocket !== 'function') {
    throw new BrowserQualificationRunnerError(
      'E_BROWSER_NETWORK_ISOLATION_UNAVAILABLE',
      'network-isolation',
      'Exact HTTP and WebSocket routing is required before qualification page creation',
      { exactOrigin },
    )
  }
  const violations = []
  const recordViolation = (channel, url) => {
    if (violations.length < MAX_WORKER_OBSERVATIONS) {
      violations.push(Object.freeze({ channel, url: truncateUtf8(url, MAX_LOG_MESSAGE_BYTES) }))
    }
  }
  await context.route('**/*', async (route) => {
    const requestUrl = route.request().url()
    if (isQualificationNetworkUrlAllowed(requestUrl, exactOrigin)) await route.continue()
    else {
      recordViolation('request', requestUrl)
      await route.abort('blockedbyclient')
    }
  })
  await context.routeWebSocket('**/*', async (webSocketRoute) => {
    const webSocketUrl = webSocketRoute.url()
    if (isQualificationWebSocketUrlAllowed(webSocketUrl, exactOrigin)) {
      webSocketRoute.connectToServer()
    } else {
      recordViolation('websocket', webSocketUrl)
      await webSocketRoute.close({ code: 1008, reason: 'qualification-network-policy' })
    }
  })
  return Object.freeze({
    exactOrigin,
    serviceWorkers: 'block',
    allowedRequestTransports: Object.freeze([
      'exact-http-loopback-origin',
      'same-origin-blob-browser-local',
      'data-browser-local',
    ]),
    allowedWebSocketTransport: 'exact-ws-loopback-origin',
    violations,
  })
}

async function runCleanPass(browser, runNumber, buildEvidence, vite, deadline, quarantine) {
  const logs = new BoundedQualificationLog()
  const startedAt = performance.now()
  const runEvidence = {
    run: runNumber,
    status: 'running',
    entryUrl: null,
    mainResponse: null,
    observedWorkers: [],
    observedWorkerCount: 0,
    workerResponses: [],
    networkIsolation: null,
    result: null,
    durationMs: null,
    logs: null,
  }
  let preview
  let context
  let page
  let failure
  try {
    preview = await startQualificationPreview({ vite, deadline, logs, quarantine })
    runEvidence.entryUrl = preview.entryUrl
    context = await acquireBrowserResource(
      deadline,
      'browser-context-' + runNumber,
      () => browser.newContext({
      serviceWorkers: 'block',
      }),
      quarantine,
      (lateContext) => lateContext.close(),
    )
    runEvidence.networkIsolation = await deadline.run(
      'network-isolation-' + runNumber,
      () => installQualificationNetworkIsolation(context, preview.origin),
    )
    page = await acquireBrowserResource(
      deadline,
      'browser-page-' + runNumber,
      () => context.newPage(),
      quarantine,
      (latePage) => latePage.close(),
    )
    const workerArtifacts = new Set(buildEvidence.workerArtifacts)
    const workerResponseByUrl = new Map()
    page.on('console', (message) => logs.add(message.type(), 'page-console', message.text(), message.location()))
    page.on('pageerror', (error) => logs.add('error', 'page-error', error.message, { name: error.name }))
    page.on('requestfailed', (request) => logs.add('warn', 'request-failed', request.url(), {
      failure: request.failure(),
      resourceType: request.resourceType(),
    }))
    page.on('worker', (worker) => {
      runEvidence.observedWorkerCount += 1
      if (runEvidence.observedWorkers.length < MAX_WORKER_OBSERVATIONS) {
        runEvidence.observedWorkers.push(truncateUtf8(worker.url(), MAX_LOG_MESSAGE_BYTES))
      }
    })
    page.on('response', (response) => {
      const artifactPath = relativeArtifactFromUrl(response.url(), preview.origin)
      if (artifactPath === undefined || !workerArtifacts.has(artifactPath)) return
      if (workerResponseByUrl.size >= MAX_WORKER_OBSERVATIONS
          && !workerResponseByUrl.has(response.url())) return
      workerResponseByUrl.set(response.url(), {
        url: truncateUtf8(response.url(), MAX_LOG_MESSAGE_BYTES),
        artifactPath,
        status: response.status(),
        contentType: truncateUtf8(response.headers()['content-type'] ?? '', 256),
        resourceType: response.request().resourceType(),
      })
    })
    const response = await deadline.run('page-navigation-' + runNumber, () => page.goto(preview.entryUrl, {
      waitUntil: 'load',
      timeout: Math.max(1, deadline.remainingMs()),
    }))
    if (response === null || !response.ok()) {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_PAGE_NAVIGATION',
        'navigation',
        'Qualification HTML entry did not return a successful response',
        { status: response?.status() ?? null },
      )
    }
    runEvidence.mainResponse = {
      status: response.status(),
      contentType: response.headers()['content-type'] ?? '',
    }
    if (!/^text\/html(?:;|$)/i.test(runEvidence.mainResponse.contentType)) {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_PAGE_CONTENT_TYPE',
        'navigation',
        'Qualification HTML entry returned the wrong content type',
        runEvidence.mainResponse,
      )
    }
    runEvidence.result = await readExactQualificationResult(page, deadline)
    if (runEvidence.networkIsolation.violations.length > 0) {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_NETWORK_POLICY_VIOLATION',
        'network-isolation',
        'Qualification page attempted a forbidden network transport',
        { violations: [...runEvidence.networkIsolation.violations] },
      )
    }
    runEvidence.workerResponses = [...workerResponseByUrl.values()]
    if (runEvidence.observedWorkerCount === 0 || runEvidence.workerResponses.length === 0) {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_WORKER_NOT_OBSERVED',
        'result',
        'No emitted Vite module Worker was observed in the actual browser realm',
        {
          observedWorkerCount: runEvidence.observedWorkerCount,
          workerResponses: runEvidence.workerResponses.length,
        },
      )
    }
    for (const workerResponse of runEvidence.workerResponses) {
      if (workerResponse.status < 200 || workerResponse.status >= 300
          || !/^(?:application|text)\/javascript(?:;|$)/i.test(workerResponse.contentType)) {
        throw new BrowserQualificationRunnerError(
          'E_BROWSER_WORKER_RESPONSE',
          'result',
          'An emitted Vite module Worker did not load as JavaScript',
          workerResponse,
        )
      }
    }
    runEvidence.status = 'passed'
  } catch (error) {
    failure = error
    runEvidence.status = 'failed'
  } finally {
    if (failure === undefined && runEvidence.networkIsolation?.violations.length > 0) {
      failure = new BrowserQualificationRunnerError(
        'E_BROWSER_NETWORK_POLICY_VIOLATION',
        'network-isolation',
        'Qualification page attempted a forbidden network transport',
        { violations: [...runEvidence.networkIsolation.violations] },
      )
      runEvidence.status = 'failed'
    }
    const cleanupFailures = []
    if (page !== undefined) {
      const error = await cleanupResource('page', () => page.close(), logs)
      if (error !== undefined) cleanupFailures.push(error)
    }
    if (context !== undefined) {
      const error = await cleanupResource('browser-context', () => context.close(), logs)
      if (error !== undefined) cleanupFailures.push(error)
    }
    if (preview !== undefined) {
      const error = await cleanupResource('vite-preview', () => preview.server.close(), logs)
      if (error !== undefined) cleanupFailures.push(error)
    }
    runEvidence.durationMs = Math.round((performance.now() - startedAt) * 1000) / 1000
    runEvidence.logs = logs.snapshot()
    failure = combinePrimaryFirst(
      failure,
      cleanupFailures,
      'Browser qualification pass failed and resource cleanup also failed',
    )
  }
  if (failure !== undefined) {
    const normalized = normalizeError(failure)
    normalized.runEvidence = runEvidence
    throw normalized
  }
  return runEvidence
}

function normalizeError(error, code = 'E_BROWSER_QUALIFICATION_INTERNAL', phase = 'runner') {
  if (error instanceof AggregateError) return error
  if (error instanceof BrowserQualificationRunnerError) return error
  return new BrowserQualificationRunnerError(
    code,
    phase,
    'Browser qualification runner failed unexpectedly',
    { cause: truncateUtf8(error instanceof Error ? error.message : String(error), MAX_LOG_MESSAGE_BYTES) },
    { cause: error },
  )
}

function serializeError(error) {
  if (error instanceof AggregateError) {
    return {
      name: error.name,
      code: 'E_BROWSER_AGGREGATE',
      phase: 'cleanup',
      message: truncateUtf8(error.message, MAX_LOG_MESSAGE_BYTES),
      details: {
        primaryFirst: true,
        errors: [...error.errors].map((item) => serializeError(item)),
      },
    }
  }
  const normalized = normalizeError(error)
  return {
    name: normalized.name,
    code: normalized.code,
    phase: normalized.phase,
    message: truncateUtf8(normalized.message, MAX_LOG_MESSAGE_BYTES),
    details: boundedValue(normalized.details),
  }
}

function newEvidence(options) {
  return {
    schemaVersion: BROWSER_QUALIFICATION_SCHEMA_VERSION,
    qualificationOnly: true,
    qualificationClaim: 'none',
    cleanRunFragment: 1,
    status: 'running',
    browser: options.browser,
    cleanRunsRequested: options.cleanRuns,
    engineTimeoutMs: options.engineTimeoutMs,
    startedAt: new Date().toISOString(),
    finishedAt: null,
    durationMs: null,
    containment: {
      cleanupDeadlineMs: CLEANUP_TIMEOUT_MS,
      lateSettlementDrainDeadlineMs: LATE_SETTLEMENT_DRAIN_TIMEOUT_MS,
      timedOutAcquisitionsQuarantined: true,
      hardKillJoinClaim: false,
      externalSupervisorRequiredForHardKillAndJoin: true,
    },
    runtime: {
      node: process.version,
      platform: process.platform,
      architecture: process.arch,
      playwright: null,
      vite: null,
      browser: null,
    },
    build: null,
    runs: [],
    logs: null,
    error: null,
  }
}

export async function runBrowserQualification(options, dependencies = {}) {
  const evidence = newEvidence(options)
  const startedAt = performance.now()
  const deadline = new EngineDeadline(options.engineTimeoutMs)
  const runnerLogs = new BoundedQualificationLog()
  const quarantine = new BrowserLateSettlementQuarantine(runnerLogs)
  let browser
  let failure
  try {
    if (options.cleanRuns !== 1) {
      throw new BrowserQualificationRunnerError(
        'E_BROWSER_CLEAN_RUN_ISOLATION',
        'arguments',
        'One runner process may execute exactly one clean-run fragment',
        {
          requested: options.cleanRuns,
          required: 1,
          externalFreshProcessRequired: true,
        },
      )
    }
    const preflight = await deadline.run('playwright-preflight', () => (
      dependencies.preflightPlaywright?.(options.browser)
        ?? preflightPlaywright(options.browser)
    ))
    evidence.runtime.playwright = preflight.metadata
    const vite = dependencies.vite ?? await deadline.run('vite-import', () => import('vite'))
    evidence.runtime.vite = { version: vite.version }
    evidence.build = await buildQualificationBundle({ vite, deadline, logs: runnerLogs })
    try {
      browser = await acquireBrowserResource(
        deadline,
        'browser-launch',
        () => preflight.browserType.launch({
          headless: true,
          timeout: Math.min(30_000, Math.max(1, deadline.remainingMs())),
        }),
        quarantine,
        (lateBrowser) => lateBrowser.close({
          reason: 'late browser qualification acquisition cleanup',
        }),
      )
    } catch (cause) {
      if (cause instanceof BrowserQualificationRunnerError) throw cause
      throw new BrowserQualificationRunnerError(
        'E_PLAYWRIGHT_BROWSER_LAUNCH',
        'browser-launch',
        'The exact Playwright-managed browser engine could not be launched',
        {
          browser: options.browser,
          cause: truncateUtf8(cause instanceof Error ? cause.message : String(cause), MAX_LOG_MESSAGE_BYTES),
        },
        { cause },
      )
    }
    const actualBrowserName = browser.browserType().name()
    if (actualBrowserName !== options.browser) {
      throw new BrowserQualificationRunnerError(
        'E_PLAYWRIGHT_BROWSER_IDENTITY',
        'browser-launch',
        'Playwright launched a different browser engine than requested',
        { requested: options.browser, actual: actualBrowserName },
      )
    }
    evidence.runtime.browser = {
      name: actualBrowserName,
      version: browser.version(),
    }
    try {
      evidence.runs.push(await runCleanPass(
        browser,
        1,
        evidence.build,
        vite,
        deadline,
        quarantine,
      ))
    } catch (error) {
      if (error?.runEvidence !== undefined) evidence.runs.push(error.runEvidence)
      throw error
    }
  } catch (error) {
    failure = normalizeError(error)
  } finally {
    const cleanupErrors = []
    if (browser !== undefined) {
      const cleanupFailure = await cleanupResource('browser', () => browser.close({
        reason: 'browser qualification runner cleanup',
      }), runnerLogs)
      if (cleanupFailure !== undefined) cleanupErrors.push(cleanupFailure)
    }
    cleanupErrors.push(...await quarantine.drain())
    failure = combinePrimaryFirst(
      failure,
      cleanupErrors,
      'Browser qualification failed and process-owned cleanup also failed',
    )
    evidence.finishedAt = new Date().toISOString()
    evidence.durationMs = Math.round((performance.now() - startedAt) * 1000) / 1000
    evidence.logs = runnerLogs.snapshot()
  }
  if (failure === undefined) evidence.status = 'passed'
  else {
    evidence.status = 'failed'
    evidence.error = serializeError(failure)
  }
  return evidence
}

function failedArgumentEvidence(error) {
  const now = new Date().toISOString()
  return {
    schemaVersion: BROWSER_QUALIFICATION_SCHEMA_VERSION,
    qualificationOnly: true,
    qualificationClaim: 'none',
    cleanRunFragment: 1,
    status: 'failed',
    browser: null,
    cleanRunsRequested: null,
    engineTimeoutMs: null,
    startedAt: now,
    finishedAt: now,
    durationMs: 0,
    runtime: null,
    build: null,
    runs: [],
    logs: null,
    error: serializeError(error),
  }
}

async function cli() {
  let evidence
  try {
    const options = parseBrowserQualificationArguments(process.argv.slice(2))
    if (options.help) {
      process.stdout.write(usage() + '\n')
      return
    }
    evidence = await runBrowserQualification(options)
  } catch (error) {
    evidence = failedArgumentEvidence(error)
  }
  process.stdout.write(JSON.stringify(evidence) + '\n')
  if (evidence.status !== 'passed') {
    process.stderr.write(JSON.stringify({
      event: 'browser-qualification-failed',
      error: evidence.error,
    }) + '\n')
    process.exitCode = 1
  }
}

const invokedPath = process.argv[1] === undefined ? '' : resolve(process.argv[1])
if (invokedPath === fileURLToPath(import.meta.url)) void cli()
