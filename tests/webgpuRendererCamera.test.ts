import { afterEach, describe, expect, it, vi } from 'vitest'
import { computePinchUpdate, projectAxesToScreen, WebGPURenderer } from '../src/services/webgpuRenderer'
import { transformPoint, type Mat4, type Vec3 } from '../src/services/math3d'

describe('WebGPURenderer close zoom', () => {
  it.each(['orthographic', 'perspective'] as const)('uses protected %s depth after restoring a close camera', projection => {
    const renderer = new WebGPURenderer()
    const internal = renderer as unknown as {
      bounds: { min: Vec3; max: Vec3; center: Vec3; radius: number }
      cameraState(): { eye: Vec3; viewProjection: Mat4 }
    }
    internal.bounds = { min: [-34, -34, 0], max: [34, 34, 10], center: [0, 0, 5], radius: 49 }
    renderer.restoreCameraState({
      yaw: Math.PI / 4, pitch: Math.atan(1 / Math.sqrt(2)), distance: 0.01,
      target: [0, 0, 5], projection,
    })

    const frame = internal.cameraState()
    // This front corner was behind the old eye and was cut by the near plane.
    const depth = transformPoint(frame.viewProjection, [34, -34, 10])[2]
    expect(depth).toBeGreaterThan(0)
    expect(depth).toBeLessThan(1)
    expect(renderer.getCameraState().distance).toBe(0.01)
    expect(renderer.canGoToPreviousView).toBe(false)
  })
})

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

  it('applies a face orientation and projection as one history action', () => {
    const renderer = new WebGPURenderer()
    const initial = renderer.getCameraState()

    renderer.setCameraPreset('front', 'orthographic')
    expect(renderer.getCameraState()).toMatchObject({ yaw: 0, pitch: 0, projection: 'orthographic' })
    expect(renderer.previousView()).toEqual(initial)
    expect(renderer.canGoToPreviousView).toBe(false)
  })

  it('notifies consumers when history becomes available or empty', () => {
    const renderer = new WebGPURenderer()
    const states: boolean[] = []
    renderer.onCameraHistoryChange = available => states.push(available)

    renderer.setView('right')
    renderer.previousView()

    expect(states).toEqual([true, false])
  })

  it('restores a captured camera without adding recovery to view history', () => {
    const renderer = new WebGPURenderer()
    const restored = renderer.restoreCameraState({
      yaw: -1.2,
      pitch: 0.4,
      distance: 125,
      target: [10, -20, 30],
      projection: 'orthographic',
    })

    expect(restored).toBe(true)
    expect(renderer.getCameraState()).toEqual({
      yaw: -1.2,
      pitch: 0.4,
      distance: 125,
      target: [10, -20, 30],
      projection: 'orthographic',
    })
    expect(renderer.canGoToPreviousView).toBe(false)
    expect(renderer.restoreCameraState(renderer.getCameraState())).toBe(false)
  })

  it('restores Previous View history after renderer lifecycle teardown', () => {
    const renderer = new WebGPURenderer()
    renderer.setProjection('orthographic')
    renderer.setView('front')
    const current = renderer.getCameraState()
    const history = renderer.getCameraHistorySnapshot()
    const availability: boolean[] = []
    renderer.onCameraHistoryChange = available => availability.push(available)

    renderer.destroy()
    expect(renderer.canGoToPreviousView).toBe(false)
    expect(renderer.restoreCameraState(current)).toBe(false)
    expect(renderer.restoreCameraHistory(history)).toBe(true)

    expect(renderer.canGoToPreviousView).toBe(true)
    expect(renderer.previousView()).toMatchObject({ projection: 'orthographic', yaw: Math.PI / 4 })
    expect(renderer.previousView()).toMatchObject({ projection: 'perspective' })
    expect(availability).toEqual([false, true, true, false])
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

describe('computePinchUpdate', () => {
  const state = { yaw: 0, pitch: 0, dist: 100, tx: 5, ty: 6, tz: 7 }

  it('zooms by the pointer-span ratio without panning when the midpoint is fixed', () => {
    const spread = computePinchUpdate(
      { x: 0, y: 0 }, { x: 0, y: 100 },
      { x: 0, y: -50 }, { x: 0, y: 150 },
      state, 400,
    )
    expect(spread.dist).toBeCloseTo(50, 10)
    expect(spread.tx).toBeCloseTo(5, 10)
    expect(spread.ty).toBeCloseTo(6, 10)
    expect(spread.tz).toBeCloseTo(7, 10)
    expect(spread.yaw).toBe(0)
    expect(spread.pitch).toBe(0)

    const squeeze = computePinchUpdate(
      { x: 0, y: -50 }, { x: 0, y: 150 },
      { x: 0, y: 0 }, { x: 0, y: 100 },
      state, 400,
    )
    expect(squeeze.dist).toBeCloseTo(200, 10)
  })

  it('pans with the midpoint using the same math as a right-drag pan', () => {
    const next = computePinchUpdate(
      { x: 0, y: 0 }, { x: 0, y: 100 },
      { x: 10, y: 20 }, { x: 10, y: 120 },
      state, 400,
    )
    const scale = 2 * 100 * Math.tan(Math.PI / 8) / 400
    expect(next.dist).toBeCloseTo(100, 10)
    // At yaw 0 / pitch 0 the screen right axis is +X and screen down is +Z.
    expect(next.tx).toBeCloseTo(5 - 10 * scale, 10)
    expect(next.ty).toBeCloseTo(6, 10)
    expect(next.tz).toBeCloseTo(7 + 20 * scale, 10)
  })

  it('clamps a single step to the same factor as a wheel step', () => {
    const collapse = computePinchUpdate(
      { x: 0, y: 0 }, { x: 0, y: 100 },
      { x: 0, y: 49.5 }, { x: 0, y: 50.5 },
      state, 400,
    )
    expect(collapse.dist).toBeCloseTo(100 * Math.E, 8)
    const explode = computePinchUpdate(
      { x: 0, y: 49.5 }, { x: 0, y: 50.5 },
      { x: 0, y: 0 }, { x: 0, y: 1000 },
      state, 400,
    )
    expect(explode.dist).toBeCloseTo(100 / Math.E, 8)
  })
})

describe('projectAxesToScreen', () => {
  const ISO_YAW = Math.PI / 4
  const ISO_PITCH = Math.atan(1 / Math.sqrt(2))

  it('at the Front view X points right, Z points up, and Y collapses away from the viewer', () => {
    const axes = projectAxesToScreen(0, 0)
    expect(axes.x.x).toBeCloseTo(1, 10)
    expect(axes.x.y).toBeCloseTo(0, 10)
    expect(axes.x.depth).toBeCloseTo(0, 10)
    expect(axes.z.x).toBeCloseTo(0, 10)
    expect(axes.z.y).toBeCloseTo(1, 10)
    expect(Math.hypot(axes.y.x, axes.y.y)).toBeCloseTo(0, 10)
    expect(axes.y.depth).toBeCloseTo(-1, 10)
  })

  it('at the Top view Z collapses toward the viewer while X stays right and Y points up', () => {
    const axes = projectAxesToScreen(0, Math.PI / 2)
    expect(Math.hypot(axes.z.x, axes.z.y)).toBeCloseTo(0, 10)
    expect(axes.z.depth).toBeCloseTo(1, 10)
    expect(axes.x.x).toBeCloseTo(1, 10)
    expect(axes.x.y).toBeCloseTo(0, 10)
    expect(axes.y.x).toBeCloseTo(0, 10)
    expect(axes.y.y).toBeCloseTo(1, 10)
  })

  it('at the Bottom view the triad matches the lookAt degenerate-pole basis (X left, Y up)', () => {
    // Regression: the analytic formula alone gave X/Y flipped 180 degrees at
    // exactly pitch = -PI/2, where lookAt() switches to its secondary-up
    // fallback (right = (-1,0,0), up = (0,1,0)).
    const axes = projectAxesToScreen(0, -Math.PI / 2)
    expect(Math.hypot(axes.z.x, axes.z.y)).toBeCloseTo(0, 10)
    expect(axes.z.depth).toBeCloseTo(-1, 10)
    expect(axes.x.x).toBeCloseTo(-1, 10)
    expect(axes.x.y).toBeCloseTo(0, 10)
    expect(axes.y.x).toBeCloseTo(0, 10)
    expect(axes.y.y).toBeCloseTo(1, 10)
  })

  it('at the Right view X collapses toward the viewer and Y points right', () => {
    const axes = projectAxesToScreen(Math.PI / 2, 0)
    expect(Math.hypot(axes.x.x, axes.x.y)).toBeCloseTo(0, 10)
    expect(axes.x.depth).toBeCloseTo(1, 10)
    expect(axes.y.x).toBeCloseTo(1, 10)
    expect(axes.z.y).toBeCloseTo(1, 10)
  })

  it('at the Back view X points left', () => {
    const axes = projectAxesToScreen(Math.PI, 0)
    expect(axes.x.x).toBeCloseTo(-1, 10)
    expect(axes.y.depth).toBeCloseTo(1, 10)
  })

  it('at ISO all axes foreshorten equally, Z is straight up, and X leans lower-right', () => {
    const axes = projectAxesToScreen(ISO_YAW, ISO_PITCH)
    const expectedLength = Math.cos(ISO_PITCH)
    expect(Math.hypot(axes.x.x, axes.x.y)).toBeCloseTo(expectedLength, 10)
    expect(Math.hypot(axes.y.x, axes.y.y)).toBeCloseTo(expectedLength, 10)
    expect(Math.hypot(axes.z.x, axes.z.y)).toBeCloseTo(expectedLength, 10)
    expect(axes.z.x).toBeCloseTo(0, 10)
    expect(axes.z.y).toBeGreaterThan(0)
    expect(axes.x.x).toBeGreaterThan(0)
    expect(axes.x.y).toBeLessThan(0)
  })

  it('is a rotation: projected axes stay unit-length and mutually orthogonal', () => {
    for (const [yaw, pitch] of [[0.3, -0.9], [-2.1, 1.2], [2.9, 0.05], [1.1, -1.5]]) {
      const axes = projectAxesToScreen(yaw, pitch)
      for (const axis of [axes.x, axes.y, axes.z]) {
        expect(Math.hypot(axis.x, axis.y, axis.depth)).toBeCloseTo(1, 10)
      }
      const dot = (a: typeof axes.x, b: typeof axes.x) => a.x * b.x + a.y * b.y + a.depth * b.depth
      expect(dot(axes.x, axes.y)).toBeCloseTo(0, 10)
      expect(dot(axes.y, axes.z)).toBeCloseTo(0, 10)
      expect(dot(axes.x, axes.z)).toBeCloseTo(0, 10)
    }
  })
})

describe('WebGPURenderer two-pointer gestures', () => {
  interface PointerLike {
    pointerId: number
    pointerType?: string
    button?: number
    shiftKey?: boolean
    clientX: number
    clientY: number
    type?: string
    preventDefault?: () => void
  }
  interface RendererInternals {
    canvas: unknown
    activePointer: number | null
    pinching: boolean
    gestureMoved: boolean
    mx: number
    my: number
    onDown: (event: PointerLike) => void
    onMove: (event: PointerLike) => void
    onPointerEnd: (event: PointerLike) => void
  }

  function makeRenderer() {
    const renderer = new WebGPURenderer()
    const internal = renderer as unknown as RendererInternals
    internal.canvas = {
      focus() {},
      setPointerCapture() {},
      releasePointerCapture() {},
      hasPointerCapture: () => false,
      clientHeight: 400,
      style: {},
    }
    return { renderer, internal }
  }

  const touch = (id: number, x: number, y: number): PointerLike => ({
    pointerId: id, pointerType: 'touch', button: 0, shiftKey: false,
    clientX: x, clientY: y, preventDefault() {},
  })

  it('enters pinch on a second touch, zooms by span ratio, and ignores a third pointer', () => {
    const { renderer, internal } = makeRenderer()
    internal.onDown(touch(1, 100, 100))
    expect(internal.activePointer).toBe(1)
    expect(internal.pinching).toBe(false)

    internal.onDown(touch(2, 200, 100))
    expect(internal.pinching).toBe(true)
    internal.onDown(touch(3, 300, 300))
    expect(internal.pinching).toBe(true)

    const before = renderer.getCameraState()
    // Pointer 2 moves from 100px to 200px span: zoom in by half.
    internal.onMove({ pointerId: 2, clientX: 300, clientY: 100 })
    expect(renderer.dist).toBeCloseTo(before.distance / 2, 8)
    expect(renderer.yaw).toBe(before.yaw)
    expect(renderer.pitch).toBe(before.pitch)
  })

  it('resumes single-pointer orbit without a jump when one finger lifts', () => {
    const { renderer, internal } = makeRenderer()
    internal.onDown(touch(1, 100, 100))
    internal.onDown(touch(2, 200, 100))
    internal.onMove({ pointerId: 2, clientX: 250, clientY: 100 })

    internal.onPointerEnd({ pointerId: 2, type: 'pointerup', clientX: 250, clientY: 100 })
    expect(internal.pinching).toBe(false)
    expect(internal.activePointer).toBe(1)
    expect(internal.mx).toBe(100)
    expect(internal.my).toBe(100)

    // A no-op move of the remaining finger must not change the camera.
    const settled = renderer.getCameraState()
    internal.onMove({ pointerId: 1, clientX: 100, clientY: 100 })
    expect(renderer.getCameraState()).toEqual(settled)

    // The remaining finger orbits from its re-anchored position.
    internal.onMove({ pointerId: 1, clientX: 110, clientY: 100 })
    expect(renderer.yaw).toBeCloseTo(settled.yaw - 10 * 0.005, 10)

    internal.onPointerEnd({ pointerId: 1, type: 'pointerup', clientX: 110, clientY: 100 })
    expect(internal.activePointer).toBeNull()
  })

  it('keeps mouse gestures on the single-pointer path', () => {
    const { internal } = makeRenderer()
    internal.onDown({ pointerId: 5, pointerType: 'mouse', button: 0, shiftKey: false, clientX: 0, clientY: 0, preventDefault() {} })
    internal.onDown({ pointerId: 6, pointerType: 'mouse', button: 2, shiftKey: false, clientX: 10, clientY: 10, preventDefault() {} })
    expect(internal.pinching).toBe(false)
    expect(internal.activePointer).toBe(5)
  })
})
