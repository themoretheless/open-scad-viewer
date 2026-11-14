import assert from 'node:assert/strict'
import test from 'node:test'
import path from 'node:path'
import { compareRuns, hostEnvironment, measureRun, median, mergeRuns, platformBaselinePath } from './rbench-regression.mjs'

function run(values = [100, 120], operations = [1, 2], caseId = 'check/tetra') {
  return {
    schema: 1,
    id: 'test',
    status: 'complete',
    environment: { arch: 'aarch64', os: 'macos' },
    cases: [{
      id: caseId,
      contract: { samples: '2', 'param.triangles': '4' },
      metrics: [{ id: 'wall', unit: 'ns', direction: 'lower' }],
    }],
    observations: values.map((value, sequence) => ({
      case: caseId, metric: 'wall', variant: 'candidate', process: 0,
      sequence, value: String(value), operations: operations[sequence], availability: { state: 'available' },
    })),
  }
}

test('normalizes batch totals and uses the median per operation', () => {
  assert.equal(median([4, 1, 3, 2]), 2.5)
  const measured = measureRun(run(), 'fixture')
  assert.deepEqual([...measured.measurements.values()], [80])
})

test('accepts changes inside the configured regression budget', () => {
  const report = compareRuns(run(), run([110, 130]), 0.25)
  assert.equal(report.ok, true)
  assert.equal(report.cases[0].regression, 0.09375)
})

test('blocks a median regression without modifying either baseline', () => {
  const baseline = run()
  const before = JSON.stringify(baseline)
  const report = compareRuns(baseline, run([220, 260]), 0.25)
  assert.equal(report.ok, false)
  assert.equal(report.cases[0].regression, 1.1875)
  assert.equal(JSON.stringify(baseline), before)
})

test('rejects missing cases and changed contracts instead of silently passing', () => {
  const missing = run()
  missing.cases = []
  assert.throws(() => compareRuns(run(), missing), /missing cases/)
  const changed = run()
  changed.cases[0].contract['param.triangles'] = '8'
  assert.throws(() => compareRuns(run(), changed), /contract differs/)
})

test('rejects environment mismatches, incomplete runs and unavailable samples', () => {
  const otherEnvironment = run()
  otherEnvironment.environment.os = 'linux'
  assert.throws(() => compareRuns(run(), otherEnvironment), /Environment mismatch/)
  const incomplete = run()
  incomplete.status = 'interrupted'
  assert.throws(() => compareRuns(run(), incomplete), /complete/)
  const unavailable = run()
  unavailable.observations[0].availability.state = 'unavailable'
  assert.throws(() => compareRuns(run(), unavailable), /unavailable/)
})

test('rejects wrong sample counts, zero batches and unsafe values', () => {
  const missingSample = run([100])
  assert.throws(() => measureRun(missingSample, 'fixture'), /expected 2 samples/)
  assert.throws(() => measureRun(run([100, 100], [1, 0]), 'fixture'), /operation count/)
  assert.throws(() => measureRun(run([100, Number.MAX_SAFE_INTEGER + 1]), 'fixture'), /batch value/)
})

test('defaults to a 15 percent budget', () => {
  assert.equal(compareRuns(run(), run([114, 136])).ok, true)
  const report = compareRuns(run(), run([120, 140]))
  assert.equal(report.maxRegression, 0.15)
  assert.equal(report.ok, false)
})

test('reports cases added since the baseline without gating them, but rejects removed cases', () => {
  const current = mergeRuns([run(), run([500, 500], [1, 1], 'hull/sphere')])
  const report = compareRuns(run(), current)
  assert.equal(report.ok, true)
  assert.deepEqual(report.cases.map(testCase => [testCase.id, Boolean(testCase.added)]), [['check/tetra', false], ['hull/sphere', true]])
  assert.throws(() => compareRuns(current, run()), /lacks: hull\/sphere/)
})

test('merges suites from one environment only', () => {
  const merged = mergeRuns([run(), run([1, 1], [1, 1], 'decimate/sphere')])
  assert.equal(merged.cases.length, 2)
  assert.equal(merged.observations.length, 4)
  const linux = run([1, 1], [1, 1], 'decimate/sphere')
  linux.environment.os = 'linux'
  assert.throws(() => mergeRuns([run(), linux]), /different environments/)
  assert.throws(() => mergeRuns([]), /No rbench suites/)
})

test('names the per-platform baseline with rbench environment names', () => {
  assert.deepEqual(hostEnvironment('darwin', 'arm64'), { os: 'macos', arch: 'aarch64' })
  assert.deepEqual(hostEnvironment('linux', 'x64'), { os: 'linux', arch: 'x86_64' })
  assert.equal(path.basename(platformBaselinePath({ os: 'linux', arch: 'x86_64' })), 'rbench-baseline.linux-x86_64.json')
})
