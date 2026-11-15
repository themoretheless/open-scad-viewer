#!/usr/bin/env node
import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { parseArgs } from 'node:util'
import { fileURLToPath } from 'node:url'

const toolDirectory = path.dirname(fileURLToPath(import.meta.url))
const repositoryRoot = path.resolve(toolDirectory, '../..')

function median(values) {
  if (values.length === 0) throw new Error('Cannot take the median of an empty sample')
  const sorted = [...values].sort((a, b) => a - b)
  const middle = Math.floor(sorted.length / 2)
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2
}

/** Convert rbench batch totals into comparable per-operation process medians. */
function measureRun(run, label) {
  if (run?.schema !== 1 || run.status !== 'complete') throw new Error(`${label}: expected a complete rbench schema 1 run`)
  if (!run.environment || typeof run.environment.arch !== 'string' || typeof run.environment.os !== 'string') {
    throw new Error(`${label}: missing environment`)
  }
  if (!Array.isArray(run.cases) || run.cases.length === 0 || !Array.isArray(run.observations)) {
    throw new Error(`${label}: missing cases or observations`)
  }
  const contracts = new Map()
  for (const testCase of run.cases) {
    if (!testCase || typeof testCase.id !== 'string' || contracts.has(testCase.id) || !testCase.contract) {
      throw new Error(`${label}: invalid or duplicate case`)
    }
    if (!Array.isArray(testCase.metrics) || !testCase.metrics.some(metric => metric.id === 'wall' && metric.unit === 'ns' && metric.direction === 'lower')) {
      throw new Error(`${label}/${testCase.id}: missing lower-is-better wall/ns metric`)
    }
    contracts.set(testCase.id, testCase.contract)
  }
  const samples = new Map()
  for (const observation of run.observations) {
    if (observation?.variant !== 'candidate' || observation.metric !== 'wall') continue
    if (!contracts.has(observation.case) || !Number.isSafeInteger(observation.process) || observation.process < 0) {
      throw new Error(`${label}: observation references an unknown case or process`)
    }
    if (observation.availability?.state !== 'available') throw new Error(`${label}/${observation.case}: unavailable wall sample`)
    const value = Number(observation.value)
    if (!Number.isSafeInteger(value) || value < 0 || !Number.isSafeInteger(observation.operations) || observation.operations < 1) {
      throw new Error(`${label}/${observation.case}: invalid batch value or operation count`)
    }
    const key = `${observation.case}\0${observation.process}`
    if (!samples.has(key)) samples.set(key, { caseId: observation.case, process: observation.process, values: [] })
    samples.get(key).values.push(value / observation.operations)
  }
  const byCase = new Map()
  for (const sample of samples.values()) {
    if (!byCase.has(sample.caseId)) byCase.set(sample.caseId, [])
    byCase.get(sample.caseId).push(median(sample.values))
  }
  const measurements = new Map()
  for (const caseId of contracts.keys()) {
    const processes = byCase.get(caseId)
    if (!processes?.length) throw new Error(`${label}/${caseId}: no available wall observations`)
    const expectedSamples = Number(contracts.get(caseId).samples)
    if (Number.isSafeInteger(expectedSamples) && expectedSamples > 0) {
      for (const sample of samples.values()) {
        if (sample.caseId === caseId && sample.values.length !== expectedSamples) {
          throw new Error(`${label}/${caseId}: expected ${expectedSamples} samples, got ${sample.values.length}`)
        }
      }
    }
    measurements.set(caseId, median(processes))
  }
  return { environment: run.environment, contracts, measurements }
}

const DEFAULT_MAX_REGRESSION = 0.15

/** Both kernel suites; a checkout that predates a suite simply lacks its example. */
const SUITES = [
  { crate: 'manifold-core', example: 'bench_manifold' },
  { crate: 'manifold-ops', example: 'bench_manifold_ops' },
]

/** rbench names for the host, matching `run.environment`. */
function hostEnvironment(platform = process.platform, arch = process.arch) {
  const os = { darwin: 'macos', linux: 'linux', win32: 'windows' }[platform] ?? platform
  const cpu = { arm64: 'aarch64', x64: 'x86_64' }[arch] ?? arch
  return { os, arch: cpu }
}

/** Committed baseline for one platform: `rbench-baseline.<os>-<arch>.json`. */
function platformBaselinePath(environment = hostEnvironment()) {
  return path.join(toolDirectory, `rbench-baseline.${environment.os}-${environment.arch}.json`)
}

/** Concatenate complete suite runs from one machine into one comparable run. */
function mergeRuns(runs) {
  if (runs.length === 0) throw new Error('No rbench suites ran')
  const [first] = runs
  for (const run of runs) {
    if (run?.schema !== 1 || run.status !== 'complete') throw new Error('Cannot merge an incomplete rbench run')
    if (run.environment?.os !== first.environment?.os || run.environment?.arch !== first.environment?.arch) {
      throw new Error('Cannot merge rbench runs from different environments')
    }
  }
  return {
    ...first,
    id: runs.map(run => run.id).join('+'),
    cases: runs.flatMap(run => run.cases),
    observations: runs.flatMap(run => run.observations),
  }
}

function compareRuns(baselineRun, currentRun, maxRegression = DEFAULT_MAX_REGRESSION) {
  if (!Number.isFinite(maxRegression) || maxRegression < 0) throw new Error('maxRegression must be a nonnegative finite ratio')
  const baseline = measureRun(baselineRun, 'baseline')
  const current = measureRun(currentRun, 'current')
  if (baseline.environment.arch !== current.environment.arch || baseline.environment.os !== current.environment.os) {
    throw new Error(`Environment mismatch: baseline=${baseline.environment.os}/${baseline.environment.arch}, current=${current.environment.os}/${current.environment.arch}`)
  }
  const baselineIds = [...baseline.contracts.keys()].sort()
  const missing = baselineIds.filter(id => !current.contracts.has(id))
  if (missing.length) throw new Error(`Benchmark case set differs from baseline; current run lacks: ${missing.join(', ')}`)
  const cases = baselineIds.map(id => {
    if (JSON.stringify(baseline.contracts.get(id)) !== JSON.stringify(current.contracts.get(id))) {
      throw new Error(`${id}: benchmark contract differs from baseline`)
    }
    const baselineNs = baseline.measurements.get(id)
    const currentNs = current.measurements.get(id)
    if (!(baselineNs > 0) || !(currentNs >= 0)) throw new Error(`${id}: invalid normalized median`)
    const regression = currentNs / baselineNs - 1
    return { id, baselineNs, currentNs, regression, ok: regression <= maxRegression }
  })
  // Cases added since the baseline are reported but cannot regress yet;
  // re-record the baseline to start gating them.
  const added = [...current.contracts.keys()].filter(id => !baseline.contracts.has(id)).sort()
    .map(id => ({ id, baselineNs: null, currentNs: current.measurements.get(id), regression: null, ok: true, added: true }))
  return { schema: 1, ok: cases.every(testCase => testCase.ok), maxRegression, environment: current.environment, cases: [...cases, ...added] }
}

async function readJson(file) {
  return JSON.parse(await readFile(path.resolve(file), 'utf8'))
}

async function runSuite(profile, root, suite) {
  const parent = path.join(toolDirectory, 'tmp')
  await mkdir(parent, { recursive: true })
  const temporary = await mkdtemp(path.join(parent, 'rbench-'))
  // rbench requires a new output directory, not an existing empty one.
  const output = path.join(temporary, 'run')
  const cargo = process.env.CARGO || 'cargo'
  const result = spawnSync(cargo, [
    'run', '--quiet', '--locked', '--release', '--manifest-path', path.join(root, 'crates/Cargo.toml'),
    '-p', suite.crate, '--example', suite.example, '--',
    '--profile', profile, '--json', '--output', output,
  ], { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 })
  if (result.error) throw result.error
  if (result.status !== 0) throw new Error(`rbench ${suite.crate}/${suite.example} failed (${result.status}):\n${result.stderr}`)
  return readJson(path.join(output, 'run.json'))
}

/** Run every suite present in the checkout at `root` and merge the runs. */
async function runBench(profile, root = repositoryRoot) {
  const runs = []
  for (const suite of SUITES) {
    if (!existsSync(path.join(root, 'crates', suite.crate, 'examples', `${suite.example}.rs`))) continue
    runs.push(await runSuite(profile, root, suite))
  }
  return mergeRuns(runs)
}

async function writeJson(file, value) {
  await mkdir(path.dirname(path.resolve(file)), { recursive: true })
  await writeFile(path.resolve(file), `${JSON.stringify(value, null, 2)}\n`)
}

const USAGE = `Usage:
  node rbench-regression.mjs record  [--baseline FILE] [--profile thorough]
  node rbench-regression.mjs check   [--baseline FILE] [--record-missing FILE] [--max-regression 0.15] [--report FILE] [--profile thorough]
  node rbench-regression.mjs measure --out RUN.json [--root CHECKOUT] [--profile thorough]
  node rbench-regression.mjs compare --baseline RUN.json --current RUN.json [--max-regression 0.15] [--report FILE]
The default baseline is rbench-baseline.<os>-<arch>.json for the current host.`

async function main() {
  const { values, positionals } = parseArgs({
    args: process.argv.slice(2),
    allowPositionals: true,
    options: {
      baseline: { type: 'string' },
      current: { type: 'string' },
      report: { type: 'string' },
      out: { type: 'string' },
      root: { type: 'string' },
      'record-missing': { type: 'string' },
      'max-regression': { type: 'string', default: String(DEFAULT_MAX_REGRESSION) },
      profile: { type: 'string', default: 'thorough' },
    },
  })
  const [command] = positionals
  if (!['record', 'check', 'compare', 'measure'].includes(command)) throw new Error(USAGE)
  if (!['quick', 'normal', 'thorough'].includes(values.profile)) throw new Error('Invalid rbench profile')
  if (command === 'compare' && (!values.current || !values.baseline)) throw new Error('compare requires --baseline and --current RUN.json')
  if (command === 'measure' && !values.out) throw new Error('measure requires --out RUN.json')
  const maxRegression = Number(values['max-regression'])
  const baselineFile = path.resolve(values.baseline || platformBaselinePath())
  if (command === 'measure') {
    const run = await runBench(values.profile, path.resolve(values.root || repositoryRoot))
    measureRun(run, 'measurement')
    await writeJson(values.out, run)
    console.log(`Measured ${run.cases.length} rbench cases into ${values.out}`)
    return
  }
  if (command === 'record') {
    const run = await runBench(values.profile)
    measureRun(run, 'new baseline')
    await writeJson(baselineFile, run)
    console.log(`Recorded ${run.cases.length} rbench cases in ${baselineFile}`)
    return
  }
  if (command === 'check' && !existsSync(baselineFile)) {
    if (!values['record-missing']) throw new Error(`No baseline at ${baselineFile}; record one first or pass --record-missing FILE`)
    const run = await runBench(values.profile)
    measureRun(run, 'new baseline')
    await writeJson(values['record-missing'], run)
    console.log(`No baseline at ${baselineFile}; recorded ${run.cases.length} cases in ${values['record-missing']} instead (nothing compared).`)
    return
  }
  const current = command === 'compare' ? await readJson(values.current) : await runBench(values.profile)
  const report = compareRuns(await readJson(baselineFile), current, maxRegression)
  const reportFile = path.resolve(values.report || path.join(toolDirectory, 'tmp', 'rbench-comparison.json'))
  await writeJson(reportFile, report)
  for (const testCase of report.cases) {
    if (testCase.added) {
      console.log(`NEW  ${testCase.id}: current=${testCase.currentNs.toFixed(1)} ns/op (not in baseline)`)
      continue
    }
    console.log(`${testCase.ok ? 'PASS' : 'FAIL'} ${testCase.id}: baseline=${testCase.baselineNs.toFixed(1)} ns/op current=${testCase.currentNs.toFixed(1)} ns/op change=${(testCase.regression * 100).toFixed(2)}%`)
  }
  console.log(`rbench regression report: ${reportFile}`)
  if (!report.ok) throw new Error(`Performance regression exceeds ${(maxRegression * 100).toFixed(2)}%`)
}

export { compareRuns, hostEnvironment, measureRun, median, mergeRuns, platformBaselinePath }
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main().catch(error => {
    console.error(error instanceof Error ? error.message : error)
    process.exitCode = 1
  })
}
