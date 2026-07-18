import { describe, expect, it } from 'vitest'
import { CameraHistory, cameraStatesEqual, type CameraState } from '../src/services/cameraHistory'

function state(distance: number, projection: CameraState['projection'] = 'perspective'): CameraState {
  return { yaw: distance / 10, pitch: 0.2, distance, target: [1, 2, 3], projection }
}

describe('camera history', () => {
  it('stores immutable snapshots and restores them newest-first', () => {
    const history = new CameraHistory()
    const mutableTarget: [number, number, number] = [1, 2, 3]
    const first: CameraState = { ...state(10), target: mutableTarget }
    history.record(first)
    mutableTarget[0] = 99
    history.record(state(20, 'orthographic'))

    expect(history.previous()).toEqual(state(20, 'orthographic'))
    expect(history.previous()).toEqual(state(10))
    expect(history.previous()).toBeNull()
  })

  it('deduplicates near-identical states and enforces its bound', () => {
    const history = new CameraHistory(2)
    expect(history.record(state(10))).toBe(true)
    expect(history.record({ ...state(10), yaw: state(10).yaw + 1e-10 })).toBe(false)
    history.record(state(20))
    history.record(state(30))

    expect(history.size).toBe(2)
    expect(history.previous()?.distance).toBe(30)
    expect(history.previous()?.distance).toBe(20)
  })

  it('rejects invalid snapshots and compares projection, target and scale safely', () => {
    const history = new CameraHistory()
    expect(history.record({ ...state(1), distance: 0 })).toBe(false)
    expect(history.record({ ...state(1), yaw: Number.NaN })).toBe(false)
    expect(cameraStatesEqual(state(10), state(10))).toBe(true)
    expect(cameraStatesEqual(state(10), state(10, 'orthographic'))).toBe(false)
    expect(cameraStatesEqual(state(10), { ...state(10), target: [1, 2, 4] })).toBe(false)
  })

  it('validates the configured capacity', () => {
    expect(() => new CameraHistory(0)).toThrow(RangeError)
    expect(() => new CameraHistory(1.5)).toThrow(RangeError)
  })
})
