import { describe, expect, it } from 'vitest'
import { BuildPerformanceHistory } from '../src/services/buildPerformance'

const sample = () => ({
  revision: 1, quality: 'preview' as const,
  phases: { parseMs: 1, bindMs: 0, initializeMs: 2, evaluateMs: 3, analyzeMs: 4 },
  workerMs: 10, hostMs: 20, publicationMs: 5, transferBytes: 128,
  upload: { geometryUploadBytes: 64, geometryBuffersCreated: 2, reusedEntities: 0 },
})

describe('bounded build measurements', () => {
  it('separates edit and publication clocks and records a submission only once', () => {
    const history = new BuildPerformanceHistory()
    history.edited(1, 10)
    const token = history.record(sample(), 40)
    expect(history.submitted(token, 50)).toBe(true)
    expect(history.submitted(token, 60)).toBe(false)
    expect(history.snapshot()[0]).toMatchObject({ editToSubmitMs: 40, publicationToSubmitMs: 10 })
  })

  it('does not attach a superseded scene frame or an unrelated edit to a new publication', () => {
    const history = new BuildPerformanceHistory()
    history.edited(1, 10)
    const old = history.record(sample(), 20)
    const next = history.record({ ...sample(), revision: 2 }, 30)
    expect(history.submitted(old, 35)).toBe(false)
    expect(history.submitted(next, 29)).toBe(false)
    expect(history.submitted(next, 40)).toBe(true)
    expect(history.snapshot()[0].publicationToSubmitMs).toBeNull()
    expect(history.snapshot()[1]).toMatchObject({ editToSubmitMs: null, publicationToSubmitMs: 10 })
  })

  it('bounds retained data and isolates snapshots from callers', () => {
    const history = new BuildPerformanceHistory()
    const input = sample()
    for (let i = 0; i < 65; i++) history.record(input, i)
    input.phases.parseMs = 999
    const snapshot = history.snapshot()
    expect(snapshot).toHaveLength(60)
    expect(snapshot[0].token).toBe(6)
    snapshot[0].phases.parseMs = 500
    snapshot[0].upload!.geometryUploadBytes = 500
    expect(history.snapshot()[0].phases.parseMs).toBe(1)
    expect(history.snapshot()[0].upload!.geometryUploadBytes).toBe(64)
    expect(history.submitted(1, 100)).toBe(false)
  })
})
