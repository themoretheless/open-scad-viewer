import { describe, expect, it } from 'vitest'
import { ViewportController } from '../src/services/viewportController'

describe('ViewportController', () => {
  it('owns face-preset projection, history availability and recovery fencing', () => {
    const viewport = new ViewportController()
    expect(viewport.state.activeView).toBe('iso')
    expect(viewport.state.camera.projection).toBe('perspective')

    const face = viewport.applyStandardView('front')
    expect(face.projection).toBe('orthographic')
    expect(viewport.state.projectionBeforeFaceSnap).toBe('perspective')

    viewport.applyCamera({
      yaw: 0.4,
      pitch: 0.2,
      distance: 120,
      target: [0, 0, 0],
      projection: 'orthographic',
    })
    expect(viewport.state.activeView).toBe('custom')
    expect(viewport.state.camera.projection).toBe('perspective')

    viewport.applyHistoryAvailability(true)
    expect(viewport.state.canGoBack).toBe(true)

    const first = viewport.beginRecovery()
    const second = viewport.beginRecovery()
    expect(viewport.isCurrentRecovery(first)).toBe(false)
    expect(viewport.isCurrentRecovery(second)).toBe(true)
    viewport.completeRecovery(second)
    expect(viewport.isRecovering).toBe(false)
  })
})
