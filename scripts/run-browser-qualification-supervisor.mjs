#!/usr/bin/env node

import { createHash } from 'node:crypto'
import { spawn } from 'node:child_process'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

export const BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT = Object.freeze({
  schema: 'browser-qualification-supervisor-evidence-v1',
  platform: 'linux',
  containment: 'detached-posix-process-group',
  actualOuterTimeoutMs: 150_000,
  memoryOuterTimeoutMs: 30 * 60_000,
  joinTimeoutMs: 5_000,
  groupDrainTimeoutMs: 2_000,
  maximumOutputBytes: 256 * 1024,
  cleanRunsRequired: 3,
})

const repositoryRoot = fileURLToPath(new URL('../', import.meta.url))
const actualRunner = fileURLToPath(new URL('./run-browser-qualification.mjs', import.meta.url))
const memoryRunner = fileURLToPath(new URL('./run-manifold-g1-memory-qualification.mjs', import.meta.url))
const admittedBrowsers = new Set(['chromium', 'firefox', 'webkit'])

export class BrowserQualificationSupervisorError extends Error {
  constructor(code, message, details = {}, options = undefined) {
    super(message, options)
    this.name = 'BrowserQualificationSupervisorError'
    this.code = code
    this.details = Object.freeze({ ...details })
  }
}

function exactInteger(raw, name, minimum, maximum) {
  if (!/^[0-9]+$/.test(raw ?? '')) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_ARGUMENT',
      `${name} must be an integer`,
      { name },
    )
  }
  const value = Number(raw)
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_ARGUMENT',
      `${name} is outside its admitted range`,
      { name, minimum, maximum },
    )
  }
  return value
}

export function parseBrowserSupervisorArguments(argv) {
  const values = new Map()
  for (let index = 0; index < argv.length; index += 1) {
    const name = argv[index]
    if (!['--mode', '--browser', '--run-index'].includes(name) || values.has(name)) {
      throw new BrowserQualificationSupervisorError(
        'E_SUPERVISOR_ARGUMENT',
        'Unknown or repeated browser supervisor argument',
        { argument: name },
      )
    }
    const value = argv[++index]
    if (value === undefined || value.startsWith('--')) {
      throw new BrowserQualificationSupervisorError(
        'E_SUPERVISOR_ARGUMENT',
        `${name} requires a value`,
        { argument: name },
      )
    }
    values.set(name, value)
  }
  const mode = values.get('--mode')
  const browser = values.get('--browser')
  if (!['actual', 'memory'].includes(mode)) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_ARGUMENT',
      '--mode must be actual or memory',
    )
  }
  if (!admittedBrowsers.has(browser)) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_ARGUMENT',
      '--browser must be chromium, firefox, or webkit',
    )
  }
  const runIndex = exactInteger(
    values.get('--run-index'),
    '--run-index',
    1,
    BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.cleanRunsRequired,
  )
  return Object.freeze({ mode, browser, runIndex })
}

function frozenChildInvocation(config) {
  if (config.mode === 'actual') {
    return Object.freeze({
      script: actualRunner,
      arguments: Object.freeze([
        '--browser', config.browser,
        '--clean-runs', '1',
        '--timeout-ms', '120000',
      ]),
      timeoutMs: BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.actualOuterTimeoutMs,
    })
  }
  return Object.freeze({
    script: memoryRunner,
    arguments: Object.freeze([
      '--surface', 'browser',
      '--browser', config.browser,
      '--cycles', '500',
      '--warmup', '50',
      '--sample-every', '25',
      '--run-index', String(config.runIndex),
    ]),
    timeoutMs: BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.memoryOuterTimeoutMs,
  })
}

function boundedDrain(stream, maximumBytes) {
  const chunks = []
  const hash = createHash('sha256')
  let byteLength = 0
  let retainedBytes = 0
  let overflow = false
  stream.on('data', chunk => {
    const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk)
    hash.update(bytes)
    byteLength += bytes.byteLength
    if (retainedBytes < maximumBytes) {
      const retained = bytes.subarray(0, maximumBytes - retainedBytes)
      chunks.push(retained)
      retainedBytes += retained.byteLength
    }
    if (byteLength > maximumBytes) overflow = true
  })
  return () => Object.freeze({
    bytes: Buffer.concat(chunks),
    byteLength,
    retainedBytes,
    overflow,
    sha256: hash.digest('hex'),
  })
}

function childSettlement(child) {
  return new Promise((resolvePromise, reject) => {
    child.once('error', reject)
    child.once('close', (code, signal) => resolvePromise(Object.freeze({ code, signal })))
  })
}

function delay(milliseconds) {
  return new Promise(resolvePromise => setTimeout(resolvePromise, milliseconds))
}

function groupExists(pid, signal = process.kill) {
  try {
    signal(-pid, 0)
    return true
  } catch (error) {
    if (error?.code === 'ESRCH') return false
    if (error?.code === 'EPERM') return true
    throw error
  }
}

async function waitForGroupExit(pid, signal = process.kill) {
  const deadline = Date.now() + BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.groupDrainTimeoutMs
  while (Date.now() < deadline) {
    if (!groupExists(pid, signal)) return
    await delay(25)
  }
  if (groupExists(pid, signal)) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_GROUP_JOIN',
      'Qualification child process group remained live after hard termination',
      { pid },
    )
  }
}

async function hardKillAndJoin(child, settlement, signal = process.kill) {
  const pid = child.pid
  if (!Number.isSafeInteger(pid) || pid < 1) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_SPAWN',
      'Qualification child did not expose a valid process identifier',
    )
  }
  try {
    signal(-pid, 'SIGKILL')
  } catch (error) {
    if (error?.code !== 'ESRCH') {
      throw new BrowserQualificationSupervisorError(
        'E_SUPERVISOR_KILL',
        'Qualification child process group could not be hard-killed',
        { pid },
        { cause: error },
      )
    }
  }
  let timer
  const outcome = await Promise.race([
    settlement.then(value => ({ tag: 'joined', value }), error => ({ tag: 'failed', error })),
    new Promise(resolvePromise => {
      timer = setTimeout(() => resolvePromise({ tag: 'timeout' }),
        BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.joinTimeoutMs)
    }),
  ])
  if (timer !== undefined) clearTimeout(timer)
  if (outcome.tag !== 'joined') {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_JOIN',
      'Qualification child did not join after hard termination',
      { pid, outcome: outcome.tag },
      outcome.tag === 'failed' ? { cause: outcome.error } : undefined,
    )
  }
  await waitForGroupExit(pid, signal)
  return outcome.value
}

function parseChildEvidence(snapshot) {
  if (snapshot.overflow) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_OUTPUT_LIMIT',
      'Qualification child stdout exceeded its bounded evidence limit',
      { maximumBytes: BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.maximumOutputBytes },
    )
  }
  const text = snapshot.bytes.toString('utf8')
  const lines = text.split(/\r?\n/u).filter(line => line.length > 0)
  if (lines.length !== 1) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_OUTPUT_PROTOCOL',
      'Qualification child must emit exactly one JSON evidence line',
      { lines: lines.length },
    )
  }
  try {
    return JSON.parse(lines[0])
  } catch (cause) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_OUTPUT_PROTOCOL',
      'Qualification child emitted invalid JSON evidence',
      {},
      { cause },
    )
  }
}

function validateChildEvidence(config, evidence) {
  if (evidence === null || typeof evidence !== 'object' || Array.isArray(evidence)
      || evidence.qualificationClaim !== 'none' || evidence.browser !== config.browser) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_CHILD_EVIDENCE',
      'Qualification child evidence did not match the supervised fragment',
    )
  }
  if (config.mode === 'actual') {
    if (evidence.status !== 'passed' || evidence.cleanRunFragment !== 1) {
      throw new BrowserQualificationSupervisorError(
        'E_SUPERVISOR_CHILD_EVIDENCE',
        'Actual-browser child did not publish one passing no-claim fragment',
      )
    }
  } else if (evidence.status !== 'single-clean-run-passed'
      || evidence.orchestration?.runIndex !== config.runIndex
      || evidence.orchestration?.cleanRunsRepresentedByThisInvocation !== 1) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_CHILD_EVIDENCE',
      'Browser-memory child did not publish the requested passing no-claim fragment',
    )
  }
  return evidence
}

export async function superviseBrowserQualification(config, dependencies = {}) {
  const platform = dependencies.platform ?? process.platform
  if (platform !== BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.platform) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_PLATFORM',
      'Frozen browser qualification containment requires Linux process groups',
      { platform },
    )
  }
  const invocation = dependencies.invocation ?? frozenChildInvocation(config)
  const spawnChild = dependencies.spawn ?? spawn
  const signal = dependencies.kill ?? process.kill
  const child = spawnChild(process.execPath, [invocation.script, ...invocation.arguments], {
    cwd: repositoryRoot,
    env: dependencies.environment ?? process.env,
    detached: true,
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  const pid = child.pid
  if (!Number.isSafeInteger(pid) || pid < 1) {
    child.kill?.('SIGKILL')
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_SPAWN',
      'Qualification child did not expose a valid process identifier',
    )
  }
  if (child.stdout === null || child.stderr === null) {
    child.kill?.('SIGKILL')
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_SPAWN',
      'Qualification child output pipes are unavailable',
    )
  }
  const finishStdout = boundedDrain(
    child.stdout,
    BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.maximumOutputBytes,
  )
  const finishStderr = boundedDrain(
    child.stderr,
    BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.maximumOutputBytes,
  )
  const settlement = childSettlement(child)
  let timer
  const outcome = await Promise.race([
    settlement.then(value => ({ tag: 'settled', value }), error => ({ tag: 'spawn-error', error })),
    new Promise(resolvePromise => {
      timer = setTimeout(() => resolvePromise({ tag: 'timeout' }), invocation.timeoutMs)
    }),
  ])
  if (timer !== undefined) clearTimeout(timer)
  if (outcome.tag === 'spawn-error') {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_SPAWN',
      'Qualification child could not be started',
      {},
      { cause: outcome.error },
    )
  }
  if (outcome.tag === 'timeout') {
    await hardKillAndJoin(child, settlement, signal)
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_TIMEOUT',
      'Qualification child exceeded its frozen outer deadline and was hard-killed and joined',
      { timeoutMs: invocation.timeoutMs, hardKilledAndJoined: true },
    )
  }

  if (groupExists(pid, signal)) {
    await hardKillAndJoin(child, settlement, signal)
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_ORPHAN_GROUP',
      'Qualification child exited while its process group remained live',
      { pid, hardKilledAndJoined: true },
    )
  }
  const stdout = finishStdout()
  const stderr = finishStderr()
  if (stderr.overflow) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_OUTPUT_LIMIT',
      'Qualification child stderr exceeded its bounded limit',
    )
  }
  if (outcome.value.code !== 0 || outcome.value.signal !== null) {
    throw new BrowserQualificationSupervisorError(
      'E_SUPERVISOR_CHILD_EXIT',
      'Qualification child exited unsuccessfully',
      {
        code: outcome.value.code,
        signal: outcome.value.signal,
        stdoutSha256: stdout.sha256,
        stderrSha256: stderr.sha256,
      },
    )
  }
  const childEvidence = validateChildEvidence(config, parseChildEvidence(stdout))
  return Object.freeze({
    schema: BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.schema,
    qualificationOnly: true,
    qualificationClaim: 'none',
    status: 'single-clean-run-passed',
    mode: config.mode,
    browser: config.browser,
    runIndex: config.runIndex,
    containment: Object.freeze({
      platform,
      kind: BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.containment,
      outerTimeoutMs: invocation.timeoutMs,
      hardKillSignal: 'SIGKILL',
      joinedBeforePublication: true,
      processGroupEmptyBeforePublication: true,
    }),
    output: Object.freeze({
      stdoutBytes: stdout.byteLength,
      stdoutSha256: stdout.sha256,
      stderrBytes: stderr.byteLength,
      stderrSha256: stderr.sha256,
    }),
    childEvidence,
  })
}

function failureEvidence(config, error) {
  return {
    schema: BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.schema,
    qualificationOnly: true,
    qualificationClaim: 'none',
    status: 'failed',
    mode: config?.mode ?? null,
    browser: config?.browser ?? null,
    runIndex: config?.runIndex ?? null,
    error: {
      name: error instanceof Error ? error.name : 'Error',
      code: typeof error?.code === 'string' ? error.code : 'E_SUPERVISOR_INTERNAL',
      message: error instanceof Error ? error.message : String(error),
      details: error?.details ?? {},
    },
  }
}

async function cli() {
  let config
  try {
    config = parseBrowserSupervisorArguments(process.argv.slice(2))
    const evidence = await superviseBrowserQualification(config)
    process.stdout.write(JSON.stringify(evidence) + '\n')
  } catch (error) {
    process.stdout.write(JSON.stringify(failureEvidence(config, error)) + '\n')
    process.exitCode = 1
  }
}

const invokedPath = process.argv[1] === undefined ? '' : resolve(process.argv[1])
if (invokedPath === fileURLToPath(import.meta.url)) void cli()
