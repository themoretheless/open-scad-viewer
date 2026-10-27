#!/usr/bin/env node
import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { parseArgs } from 'node:util'
import { fileURLToPath } from 'node:url'

const toolDirectory = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(toolDirectory, '../..')

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

function compareRuns(baselineRun, currentRun, maxRegression = 0.25) {
  if (!Number.isFinite(maxRegression) || maxRegression < 0) throw new Error('maxRegression must be a nonnegative finite ratio')
  const baseline = measureRun(baselineRun, 'baseline')
  const current = measureRun(currentRun, 'current')
  if (baseline.environment.arch !== current.environment.arch || baseline.environment.os !== current.environment.os) {
    throw new Error(`Environment mismatch: baseline=${baseline.environment.os}/${baseline.environment.arch}, current=${current.environment.os}/${current.environment.arch}`)
  }
  const baselineIds = [...baseline.contracts.keys()].sort()
  const currentIds = [...current.contracts.keys()].sort()
  if (JSON.stringify(baselineIds) !== JSON.stringify(currentIds)) throw new Error('Benchmark case set differs from baseline')
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
  return { schema: 1, ok: cases.every(testCase => testCase.ok), maxRegression, environment: current.environment, cases }
}

async function readJson(file) {
  return JSON.parse(await readFile(path.resolve(file), 'utf8'))
}

async function runBench(profile) {
  const parent = path.join(toolDirectory, 'tmp')
  await mkdir(parent, { recursive: true })
  const temporary = await mkdtemp(path.join(parent, 'rbench-'))
  // rbench requires a new output directory, not an existing empty one.
  const output = path.join(temporary, 'run')
  const cargo = process.env.CARGO || 'cargo'
  const result = spawnSync(cargo, [
    'run', '--quiet', '--locked', '--release', '--manifest-path', 'crates/Cargo.toml',
    '-p', 'manifold-core', '--example', 'bench_manifold', '--',
    '--profile', profile, '--json', '--output', output,
  ], { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 })
  if (result.error) throw result.error
  if (result.status !== 0) throw new Error(`rbench failed (${result.status}):\n${result.stderr}`)
  return readJson(path.join(output, 'run.json'))
}

async function main() {
  const { values, positionals } = parseArgs({
    args: process.argv.slice(2),
    allowPositionals: true,
    options: {
      baseline: { type: 'string' },
      current: { type: 'string' },
      report: { type: 'string' },
      'max-regression': { type: 'string', default: '0.25' },
      profile: { type: 'string', default: 'quick' },
    },
  })
  const [command] = positionals
  if (!['record', 'check', 'compare'].includes(command) || !values.baseline) {
    throw new Error('Usage: node rbench-regression.mjs record|check|compare --baseline FILE [--current RUN.json] [--report FILE] [--max-regression 0.25] [--profile quick]')
  }
  if (!['quick', 'normal', 'thorough'].includes(values.profile)) throw new Error('Invalid rbench profile')
  if (command === 'compare' && !values.current) throw new Error('compare requires --current RUN.json')
  const maxRegression = Number(values['max-regression'])
  if (command === 'record') {
    const run = await runBench(values.profile)
    measureRun(run, 'new baseline')
    await writeFile(path.resolve(values.baseline), `${JSON.stringify(run, null, 2)}\n`)
    console.log(`Recorded ${run.cases.length} rbench cases in ${values.baseline}`)
    return
  }
  const current = command === 'compare' ? await readJson(values.current) : await runBench(values.profile)
  const report = compareRuns(await readJson(values.baseline), current, maxRegression)
  const reportFile = path.resolve(values.report || path.join(toolDirectory, 'tmp', 'rbench-comparison.json'))
  await mkdir(path.dirname(reportFile), { recursive: true })
  await writeFile(reportFile, `${JSON.stringify(report, null, 2)}\n`)
  for (const testCase of report.cases) {
    console.log(`${testCase.ok ? 'PASS' : 'FAIL'} ${testCase.id}: baseline=${testCase.baselineNs.toFixed(1)} ns/op current=${testCase.currentNs.toFixed(1)} ns/op change=${(testCase.regression * 100).toFixed(2)}%`)
  }
  console.log(`rbench regression report: ${reportFile}`)
  if (!report.ok) throw new Error(`Performance regression exceeds ${(maxRegression * 100).toFixed(2)}%`)
}

export { compareRuns, measureRun, median }
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main().catch(error => {
    console.error(error instanceof Error ? error.message : error)
    process.exitCode = 1
  })
}
