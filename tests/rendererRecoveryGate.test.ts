import { describe, expect, it } from 'vitest'
import { RendererRecoveryGate } from '../src/services/rendererRecoveryGate'

describe('RendererRecoveryGate', () => {
  it('starts one recovery and coalesces losses into one follow-up', () => {
    const gate = new RendererRecoveryGate()

    expect(gate.registerDeviceLoss()).toBe('start')
    expect(gate.isActive).toBe(true)
    expect(gate.mustDeferReady).toBe(false)

    expect(gate.registerDeviceLoss()).toBe('latched')
    expect(gate.registerDeviceLoss()).toBe('latched')
    expect(gate.mustDeferReady).toBe(true)
    expect(gate.completeAttempt()).toBe('retry')
    expect(gate.isActive).toBe(true)
    expect(gate.mustDeferReady).toBe(false)

    expect(gate.completeAttempt()).toBe('done')
    expect(gate.isActive).toBe(false)
  })

  it('stops after a replacement device is also lost and permits a later burst', () => {
    const gate = new RendererRecoveryGate()

    expect(gate.registerDeviceLoss()).toBe('start')
    expect(gate.registerDeviceLoss()).toBe('latched')
    expect(gate.completeAttempt()).toBe('retry')
    expect(gate.registerDeviceLoss()).toBe('exhausted')
    expect(gate.mustDeferReady).toBe(true)
    expect(gate.completeAttempt()).toBe('exhausted')
    expect(gate.isActive).toBe(false)

    expect(gate.registerDeviceLoss()).toBe('start')
  })

  it('can be reset when the renderer owner is disposed', () => {
    const gate = new RendererRecoveryGate()
    gate.registerDeviceLoss()
    gate.registerDeviceLoss()

    gate.reset()

    expect(gate.isActive).toBe(false)
    expect(gate.mustDeferReady).toBe(false)
    expect(gate.completeAttempt()).toBe('done')
  })
})
