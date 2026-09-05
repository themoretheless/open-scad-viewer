import { afterEach, describe, expect, it, vi } from 'vitest'
import { AutoBuildScheduler } from '../src/services/autoBuildScheduler'

function harness() {
  vi.useFakeTimers()
  vi.setSystemTime(0)
  const build = vi.fn()
  const scheduler = new AutoBuildScheduler({ build, now: () => Date.now() })
  return { scheduler, build }
}

describe('adaptive automatic builds', () => {
  afterEach(() => vi.useRealTimers())

  it('coalesces typing and lowers warm small-model latency without unbounded delays', () => {
    const { scheduler, build } = harness()
    scheduler.observe('preview', 3)
    scheduler.edited()
    vi.advanceTimersByTime(40)
    scheduler.edited()
    vi.advanceTimersByTime(59)
    expect(build).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect(build.mock.calls).toEqual([['preview']])
    scheduler.observe('preview', 10_000)
    expect(scheduler.delays).toEqual({ textMs: 450, dragMs: 750 })
    scheduler.observe('preview', NaN)
    expect(scheduler.delays.textMs).toBe(450)
  })

  it('throttles continuous drags instead of postponing previews and defers full until release', () => {
    const { scheduler, build } = harness()
    scheduler.observe('preview', 10)
    scheduler.beginGesture()
    scheduler.edited()
    vi.advanceTimersByTime(0)
    scheduler.promote()
    expect(build.mock.calls).toEqual([['preview']])
    for (let i = 0; i < 5; i++) {
      vi.advanceTimersByTime(19)
      scheduler.edited()
    }
    expect(build).toHaveBeenCalledTimes(1)
    vi.advanceTimersByTime(5)
    expect(build.mock.calls).toEqual([['preview'], ['preview']])
    scheduler.promote()
    scheduler.endGesture()
    expect(build.mock.calls.at(-1)).toEqual(['full'])
    scheduler.endGesture()
    expect(build).toHaveBeenCalledTimes(3)
  })

  it('does not repeatedly preempt expensive work while dragging, but flushes latest input on release', () => {
    const { scheduler, build } = harness()
    scheduler.beginGesture()
    scheduler.setBuilding(true)
    for (let i = 0; i < 20; i++) {
      scheduler.edited()
      vi.advanceTimersByTime(100)
    }
    expect(build).not.toHaveBeenCalled()
    scheduler.endGesture()
    expect(build.mock.calls).toEqual([['preview']])
    scheduler.promote()
    expect(build.mock.calls.at(-1)).toEqual(['full'])
  })

  it('continues with the latest drag edit when the preceding job finishes', () => {
    const { scheduler, build } = harness()
    scheduler.beginGesture()
    scheduler.setBuilding(true)
    scheduler.edited()
    scheduler.setBuilding(false)
    vi.advanceTimersByTime(0)
    expect(build.mock.calls).toEqual([['preview']])
  })

  it('cancels pending builds and promotion on disable/disposal and ignores unrelated pointer releases', () => {
    const { scheduler, build } = harness()
    scheduler.edited()
    scheduler.endGesture()
    expect(build).not.toHaveBeenCalled()
    scheduler.cancel()
    vi.runAllTimers()
    expect(build).not.toHaveBeenCalled()
    scheduler.beginGesture()
    scheduler.promote()
    scheduler.cancel()
    scheduler.endGesture()
    expect(build).not.toHaveBeenCalled()
  })

  it('does not promote a preview when a more recent source is pending', () => {
    const { scheduler, build } = harness()
    scheduler.edited()
    scheduler.promote()
    expect(build).not.toHaveBeenCalled()
    scheduler.flush()
    expect(build.mock.calls).toEqual([['preview']])
  })
})
