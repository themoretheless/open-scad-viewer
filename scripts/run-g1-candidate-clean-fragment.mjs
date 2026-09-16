#!/usr/bin/env node
/**
 * Execute one G1 v24 clean-run fragment and append evidence.
 *
 * One fragment = one (matrixRowId, environmentId, runIndex).
 * Does not invent pass/fail: records the real process exit and digests.
 *
 * Usage:
 *   node scripts/run-g1-candidate-clean-fragment.mjs \
 *     --row mcp-supervisor-hard-kill \
 *     --env macos-node20 \
 *     --run-index 1 \
 *     [--node-bin /path/to/node] \
 *     [--workdir /path/to/clean/checkout] \
 *     [--classification clean-post-freeze|discovery-only]
 */

import { createHash } from 'node:crypto'
import { spawn } from 'node:child_process'
import {
  mkdirSync,
  readFileSync,
  writeFileSync,
  appendFileSync,
  existsSync,
  openSync,
  closeSync,
  fsyncSync,
} from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const repositoryRoot = fileURLToPath(new URL('../', import.meta.url))
const PLAN_REL = 'docs/qualification/semantic-manifold-g1-plan-v24.json'
const RESULT_REL = 'output/qualification/semantic-manifold-g1-candidate-run-v24/result.json'
const FRAGMENTS_REL = 'output/qualification/semantic-manifold-g1-candidate-run-v24/fragments.jsonl'

const FORBIDDEN_ENV = [
  'NODE_OPTIONS',
  'NODE_PATH',
  'NODE_PRESERVE_SYMLINKS',
  'NPM_CONFIG_PREFIX',
  'NPM_CONFIG_CACHE',
  'npm_config_registry',
  'HTTP_PROXY',
  'HTTPS_PROXY',
  'ALL_PROXY',
  'http_proxy',
  'https_proxy',
  'all_proxy',
  'PLAYWRIGHT_BROWSERS_PATH',
  'PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD',
  'LD_PRELOAD',
  'DYLD_INSERT_LIBRARIES',
]

function die(message, code = 2) {
  console.error(message)
  process.exit(code)
}

function parseArgs(argv) {
  const values = new Map()
  for (let i = 0; i < argv.length; i += 1) {
    const name = argv[i]
    if (!name.startsWith('--')) die(`Unknown argument: ${name}`)
    const value = argv[++i]
    if (value === undefined || value.startsWith('--')) die(`${name} requires a value`)
    values.set(name, value)
  }
  const row = values.get('--row')
  const env = values.get('--env')
  const runIndex = Number(values.get('--run-index'))
  const nodeBin = values.get('--node-bin') ?? process.execPath
  const workdir = values.get('--workdir') ?? repositoryRoot
  const classification = values.get('--classification') ?? 'discovery-only'
  if (!row || !env || !Number.isInteger(runIndex) || runIndex < 1) {
    die('Required: --row <id> --env <id> --run-index <n>')
  }
  if (!['clean-post-freeze', 'discovery-only'].includes(classification)) {
    die('--classification must be clean-post-freeze or discovery-only')
  }
  return { row, env, runIndex, nodeBin, workdir, classification }
}

function sha256File(path) {
  const bytes = readFileSync(path)
  return {
    algorithm: 'sha256',
    value: createHash('sha256').update(bytes).digest('hex'),
    byteLength: bytes.byteLength,
  }
}

function sha256Text(text) {
  const bytes = Buffer.from(text, 'utf8')
  return {
    algorithm: 'sha256',
    value: createHash('sha256').update(bytes).digest('hex'),
    byteLength: bytes.byteLength,
  }
}

function loadPlan(workdir) {
  return JSON.parse(readFileSync(resolve(workdir, PLAN_REL), 'utf8'))
}

function resolveRow(plan, rowId, envId, runIndex) {
  const row = plan.matrix.find((item) => item.id === rowId)
  if (!row) die(`Unknown matrix row: ${rowId}`)
  if (!row.executionEnvironmentIds.includes(envId)) {
    die(`Row ${rowId} does not admit environment ${envId}`)
  }
  const env = plan.environments.find((item) => item.id === envId)
  if (!env) die(`Unknown environment: ${envId}`)
  if (runIndex > row.work.cleanRunsRequired) {
    die(`run-index ${runIndex} exceeds cleanRunsRequired ${row.work.cleanRunsRequired}`)
  }
  const seed = row.work.seeds[runIndex - 1]
  return { row, env, seed }
}

function sanitizeEnv(seed) {
  const base = {
    HOME: process.env.HOME,
    USER: process.env.USER,
    LOGNAME: process.env.LOGNAME,
    TMPDIR: process.env.TMPDIR,
    TMP: process.env.TMP,
    TEMP: process.env.TEMP,
    LANG: process.env.LANG ?? 'C.UTF-8',
    PATH: process.env.PATH,
    TERM: process.env.TERM ?? 'dumb',
    CI: '1',
    QUALIFICATION_SEED: seed,
    QUALIFICATION_PLAN_ID: 'semantic-manifold-g1-plan-v24',
  }
  for (const key of Object.keys(base)) {
    if (base[key] === undefined) delete base[key]
  }
  for (const key of FORBIDDEN_ENV) {
    if (process.env[key]) {
      // Explicitly omit; also fail closed if classification claims clean.
    }
  }
  return base
}

function forbiddenPresent() {
  return FORBIDDEN_ENV.filter((key) => process.env[key] !== undefined)
}

function runCommand(command, args, { cwd, env, timeoutMs }) {
  return new Promise((resolvePromise) => {
    const child = spawn(command, args, {
      cwd,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    })
    const chunks = { stdout: [], stderr: [] }
    let stdoutBytes = 0
    let stderrBytes = 0
    const max = 256 * 1024
    const onData = (stream) => (buf) => {
      const room = max - (stream === 'stdout' ? stdoutBytes : stderrBytes)
      if (room <= 0) return
      const slice = buf.subarray(0, room)
      chunks[stream].push(slice)
      if (stream === 'stdout') stdoutBytes += slice.byteLength
      else stderrBytes += slice.byteLength
    }
    child.stdout.on('data', onData('stdout'))
    child.stderr.on('data', onData('stderr'))
    let timedOut = false
    const timer = setTimeout(() => {
      timedOut = true
      child.kill('SIGKILL')
    }, timeoutMs)
    child.on('close', (code, signal) => {
      clearTimeout(timer)
      resolvePromise({
        exitCode: code,
        signal,
        timedOut,
        stdout: Buffer.concat(chunks.stdout),
        stderr: Buffer.concat(chunks.stderr),
      })
    })
  })
}

function expandCommand(template, { runIndex, browser }) {
  return template
    .replaceAll('<1|2|3>', String(runIndex))
    .replaceAll('<chromium|webkit>', browser ?? 'chromium')
    .replaceAll('<chromium|firefox|webkit>', browser ?? 'chromium')
}

function commandArgs(harnessCommand, ctx) {
  const expanded = expandCommand(harnessCommand, ctx)
  if (expanded.startsWith('npm test -- ')) {
    return {
      command: 'npm',
      args: ['test', '--ignore-scripts', '--', ...expanded.slice('npm test -- '.length).split(/\s+/).filter(Boolean)],
    }
  }
  if (expanded.startsWith('node ')) {
    const parts = expanded.split(/\s+/).filter(Boolean)
    return { command: parts[0], args: parts.slice(1) }
  }
  die(`Unsupported harness command: ${harnessCommand}`)
}

function ensureResultScaffold(resultPath, plan) {
  if (existsSync(resultPath)) {
    return JSON.parse(readFileSync(resultPath, 'utf8'))
  }
  mkdirSync(dirname(resultPath), { recursive: true })
  const scaffold = {
    schema: 'open-scad-viewer/qualification-candidate-result',
    schemaVersion: 1,
    candidateRunId: plan.executionProtocol.candidateRunId,
    planId: plan.planId,
    planPath: PLAN_REL,
    createdAt: new Date().toISOString(),
    priorResultsImported: false,
    plannedWorkUnits: plan.executionProtocol.plannedWorkUnits,
    completedWorkUnits: 0,
    status: 'in-progress',
    qualificationClaim: 'none',
    fragments: [],
    notes: [
      'Append-only result scaffold. Fragments are also mirrored to fragments.jsonl.',
      'Only classification=clean-post-freeze fragments may close u07; discovery-only is supplemental.',
    ],
  }
  writeFileSync(resultPath, `${JSON.stringify(scaffold, null, 2)}\n`)
  return scaffold
}

function appendFragment(resultPath, fragmentsPath, fragment) {
  mkdirSync(dirname(fragmentsPath), { recursive: true })
  appendFileSync(fragmentsPath, `${JSON.stringify(fragment)}\n`)
  const result = JSON.parse(readFileSync(resultPath, 'utf8'))
  result.fragments.push(fragment)
  result.completedWorkUnits = result.fragments
    .filter((item) => item.classification === 'clean-post-freeze' && item.status === 'passed')
    .reduce((sum, item) => sum + Number(item.unitsCompleted ?? 0), 0)
  result.updatedAt = new Date().toISOString()
  const text = `${JSON.stringify(result, null, 2)}\n`
  writeFileSync(resultPath, text)
  const fd = openSync(resultPath, 'r+')
  try {
    fsyncSync(fd)
  } finally {
    closeSync(fd)
  }
  return result
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const plan = loadPlan(args.workdir)
  const { row, env, seed } = resolveRow(plan, args.row, args.env, args.runIndex)

  const forbidden = forbiddenPresent()
  if (args.classification === 'clean-post-freeze' && forbidden.length > 0) {
    die(
      `Refuse clean-post-freeze while forbidden env vars are set: ${forbidden.join(', ')}. Unset them or use --classification discovery-only.`,
    )
  }

  const resultPath = resolve(repositoryRoot, RESULT_REL)
  const fragmentsPath = resolve(repositoryRoot, FRAGMENTS_REL)
  ensureResultScaffold(resultPath, plan)

  const browser = env.browser?.engine ?? null
  const invoked = commandArgs(row.harness.command, { runIndex: args.runIndex, browser })
  const envVars = sanitizeEnv(seed)
  // Prefer the explicit node binary on PATH for npm scripts.
  envVars.PATH = `${dirname(args.nodeBin)}:${envVars.PATH ?? ''}`
  envVars.npm_config_user_agent = undefined

  const startedAt = new Date().toISOString()
  const startedMs = Date.now()
  const outcome = await runCommand(invoked.command, invoked.args, {
    cwd: args.workdir,
    env: envVars,
    timeoutMs: 30 * 60 * 1000,
  })
  const endedAt = new Date().toISOString()
  const passed = !outcome.timedOut && outcome.exitCode === 0

  const stdoutDigest = sha256Text(outcome.stdout.toString('utf8'))
  const stderrDigest = sha256Text(outcome.stderr.toString('utf8'))
  const nodeDigest = sha256File(args.nodeBin)

  const fragment = {
    fragmentId: `${row.id}/${env.id}/run-${args.runIndex}`,
    matrixRowId: row.id,
    environmentId: env.id,
    runIndex: args.runIndex,
    seed,
    classification: args.classification,
    status: passed ? 'passed' : 'failed',
    unitsPlanned: row.work.unitsPerCleanRun,
    unitsCompleted: passed ? row.work.unitsPerCleanRun : 0,
    startedAt,
    endedAt,
    durationMs: Date.now() - startedMs,
    command: [invoked.command, ...invoked.args],
    exitCode: outcome.exitCode,
    signal: outcome.signal,
    timedOut: outcome.timedOut,
    nodeBinary: {
      path: args.nodeBin,
      version: process.version,
      digest: nodeDigest,
    },
    host: {
      platform: process.platform,
      arch: process.arch,
      osClaimedByPlan: env.os,
      architectureClaimedByPlan: env.architecture,
    },
    stdoutDigest,
    stderrDigest,
    forbiddenEnvPresent: forbidden,
    protocolNotes: [
      args.classification === 'clean-post-freeze'
        ? 'Caller asserted clean-post-freeze; executor verified forbidden-env absence only.'
        : 'discovery-only: may not satisfy u07 even if passed.',
      'Full cleanRunDefinition (fresh OS job, wiped caches, archive Node digest, npm ci) remains caller responsibility.',
    ],
  }

  const result = appendFragment(resultPath, fragmentsPath, fragment)
  console.log(JSON.stringify({
    fragmentId: fragment.fragmentId,
    status: fragment.status,
    classification: fragment.classification,
    completedWorkUnits: result.completedWorkUnits,
    plannedWorkUnits: result.plannedWorkUnits,
    resultPath: RESULT_REL,
  }, null, 2))
  process.exit(passed ? 0 : 1)
}

main().catch((error) => {
  console.error(error)
  process.exit(1)
})
