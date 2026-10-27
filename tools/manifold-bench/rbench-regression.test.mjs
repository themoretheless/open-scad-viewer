import assert from 'node:assert/strict'
import test from 'node:test'
import { compareRuns, measureRun, median } from './rbench-regression.mjs'

function run(values = [100, 120], operations = [1, 2]) {
  return {
    schema: 1,
    id: 'test',
    status: 'complete',
    environment: { arch: 'aarch64', os: 'macos' },
    cases: [{
      id: 'check/tetra',
      contract: { samples: '2', 'param.triangles': '4' },
      metrics: [{ id: 'wall', unit: 'ns', direction: 'lower' }],
    }],
    observations: values.map((value, sequence) => ({
      case: 'check/tetra', metric: 'wall', variant: 'candidate', process: 0,
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
