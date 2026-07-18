import { afterEach, describe, expect, it, vi } from 'vitest'
import { WebGPURenderer } from '../src/services/webgpuRenderer'

describe('WebGPURenderer camera history integration', () => {
  afterEach(() => vi.unstubAllGlobals())
  it('ignores no-op actions and restores view changes newest-first', () => {
    const renderer = new WebGPURenderer()

    renderer.fitView()
    renderer.resetView()
    renderer.setView('iso')
    expect(renderer.canGoToPreviousView).toBe(false)

    renderer.setProjection('orthographic')
    renderer.setView('front')
    expect(renderer.canGoToPreviousView).toBe(true)

    const previousAngle = renderer.previousView()
    expect(previousAngle).toMatchObject({
      projection: 'orthographic',
      yaw: Math.PI / 4,
    })

    const previousProjection = renderer.previousView()
    expect(previousProjection?.projection).toBe('perspective')
    expect(renderer.getCameraState().projection).toBe('perspective')
    expect(renderer.canGoToPreviousView).toBe(false)
    expect(renderer.previousView()).toBeNull()
  })

  it('notifies consumers when history becomes available or empty', () => {
    const renderer = new WebGPURenderer()
    const states: boolean[] = []
    renderer.onCameraHistoryChange = available => states.push(available)

    renderer.setView('right')
    renderer.previousView()

    expect(states).toEqual([true, false])
  })

  it('keeps camera snapshots chronological when an action interrupts a drag', () => {
    const renderer = new WebGPURenderer()
    const internal = renderer as unknown as {
      gestureCameraStart: ReturnType<WebGPURenderer['getCameraState']> | null
      activePointer: number | null
      drag: boolean
      pan: boolean
      gestureMoved: boolean
      downX: number
      downY: number
      downButton: number
      onPointerEnd: (event: Partial<PointerEvent>) => void
    }
    const initial = renderer.getCameraState()
    internal.gestureCameraStart = initial
    internal.activePointer = 7
    internal.drag = true
    internal.pan = false
    internal.gestureMoved = true
    renderer.yaw = 1

    renderer.setProjection('orthographic')
    renderer.yaw = 2
    internal.onPointerEnd({
      pointerId: 7,
      type: 'pointerup',
      clientX: 0,
      clientY: 0,
    })

    expect(renderer.previousView()).toMatchObject({ yaw: 1, projection: 'orthographic' })
    expect(renderer.previousView()).toMatchObject({ yaw: 1, projection: 'perspective' })
    expect(renderer.previousView()).toMatchObject({ yaw: initial.yaw, projection: 'perspective' })
  })

  it('invalidates stale depth-cycle metadata when the camera changes', () => {
    const renderer = new WebGPURenderer()
    const internal = renderer as unknown as {
      selectedHit: {
        meshIndex: number
        triangleIndex: number
        faceId: null
        point: [number, number, number]
        normal: [number, number, number]
        barycentric: [number, number, number]
        source: null
        backside: boolean
        cycleIndex: number
        cycleCount: number
      }
    }
    internal.selectedHit = {
      meshIndex: 0,
      triangleIndex: 0,
      faceId: null,
      point: [0, 0, 0],
      normal: [0, 0, 1],
      barycentric: [1, 0, 0],
      source: null,
      backside: false,
      cycleIndex: 1,
      cycleCount: 2,
    }

    renderer.setView('front')

    expect(renderer.currentHit?.cycleIndex).toBeUndefined()
    expect(renderer.currentHit?.cycleCount).toBeUndefined()
  })

  it('does not coalesce a wheel burst across an intervening drag segment', () => {
    vi.stubGlobal('WheelEvent', { DOM_DELTA_LINE: 1, DOM_DELTA_PAGE: 2 })
    const renderer = new WebGPURenderer()
    const internal = renderer as unknown as {
      gestureCameraStart: ReturnType<WebGPURenderer['getCameraState']> | null
      activePointer: number | null
      drag: boolean
      pan: boolean
      gestureMoved: boolean
      mx: number
      my: number
      downX: number
      downY: number
      downButton: number
      onMove: (event: Partial<PointerEvent>) => void
      onWheel: (event: Partial<WheelEvent>) => void
      onPointerEnd: (event: Partial<PointerEvent>) => void
    }
    const initial = renderer.getCameraState()
    internal.gestureCameraStart = initial
    internal.activePointer = 9
    internal.drag = true
    internal.pan = false
    internal.gestureMoved = false
    internal.mx = 0
    internal.my = 0
    internal.downX = 0
    internal.downY = 0
    internal.downButton = 0
    const pointer = (x: number) => ({ pointerId: 9, clientX: x, clientY: 0, preventDefault() {} })
    const wheel = { deltaMode: 0, deltaY: 100, preventDefault() {} }

    internal.onMove(pointer(4))
    const beforeFirstWheel = renderer.getCameraState()
    internal.onWheel(wheel)
    const afterFirstWheel = renderer.getCameraState()
    internal.onMove(pointer(8))
    const beforeSecondWheel = renderer.getCameraState()
    internal.onWheel(wheel)
    internal.onPointerEnd({ pointerId: 9, type: 'pointerup', clientX: 8, clientY: 0 })

    expect(renderer.previousView()).toEqual(beforeSecondWheel)
    expect(renderer.previousView()).toEqual(afterFirstWheel)
    expect(renderer.previousView()).toEqual(beforeFirstWheel)
    expect(renderer.previousView()).toEqual(initial)
  })
})
