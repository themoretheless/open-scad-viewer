import { callGeometryRust } from './geometry/kernel'
const FOV_Y = Math.PI / 4

export interface PinchPoint { x: number; y: number }

export interface OrbitCameraState {
  yaw: number
  pitch: number
  dist: number
  tx: number
  ty: number
  tz: number
}

export type PinchCameraState = OrbitCameraState

// Binary transport is finite-only; retain scalar fallback semantics explicitly.
function number(value: number): number | string { return Number.isFinite(value) ? value : String(value) }

export function clampGestureDistance(distance: number): number {
  return callGeometryRust('camera_gesture', { action: 'clamp', distance: number(distance) })
}
export function computeOrbitUpdate(state: OrbitCameraState, dx: number, dy: number): OrbitCameraState {
  return callGeometryRust('camera_gesture', { action: 'orbit', state, dx, dy })
}
export function computePanUpdate(state: OrbitCameraState, dx: number, dy: number, viewportHeight: number, fovY = FOV_Y): OrbitCameraState {
  return callGeometryRust('camera_gesture', { action: 'pan', state, dx, dy, height: viewportHeight, fov: fovY })
}
export function computeWheelDistance(distance: number, deltaPixels: number): number {
  return callGeometryRust('camera_gesture', { action: 'wheel', distance: number(distance), delta: number(deltaPixels) })
}
export function computePinchUpdate(
  prevA: PinchPoint, prevB: PinchPoint, curA: PinchPoint, curB: PinchPoint,
  state: PinchCameraState, viewportHeight: number, fovY = FOV_Y,
): PinchCameraState {
  return callGeometryRust('camera_gesture', {
    action: 'pinch', points: [prevA, prevB, curA, curB].map(p => [p.x, p.y]),
    state, height: viewportHeight, fov: fovY,
  })
}

export interface InertialOrbitVelocity {
  yawRadPerSec: number
  pitchRadPerSec: number
  zoomPerSec: number
}

const MAX_PITCH_RAD = Math.PI / 2 - 0.01
const MIN_CAMERA_DIST = 0.05
const MAX_CAMERA_DIST = 100_000

/**
 * Computes a bounded pinch-to-zoom distance multiplier from previous and current
 * two-finger touch distances in pixels.
 * When `nextDistancePx > prevDistancePx` (spreading fingers apart to zoom in),
 * the returned distance factor `prevDistancePx / nextDistancePx` is `< 1`.
 */
export function computePinchZoomFactor(prevDistancePx: number, nextDistancePx: number): number {
  if (!Number.isFinite(prevDistancePx) || !Number.isFinite(nextDistancePx) || prevDistancePx <= 1e-3 || nextDistancePx <= 1e-3) {
    return 1
  }
  const rawFactor = prevDistancePx / nextDistancePx
  return Math.max(0.1, Math.min(10, rawFactor))
}

/**
 * Advances an orbit camera state with exponential inertial velocity damping over `dtSec`.
 * Returns the updated `state`, decayed `velocity`, and `moving` flag indicating whether
 * residual motion remains above threshold.
 */
export function stepInertialOrbitCamera(
  state: OrbitCameraState,
  velocity: InertialOrbitVelocity,
  dtSec: number,
  damping = 8,
): { state: OrbitCameraState; velocity: InertialOrbitVelocity; moving: boolean } {
  if (!Number.isFinite(dtSec) || dtSec <= 0) {
    return {
      state: { ...state },
      velocity: { ...velocity },
      moving:
        Math.hypot(velocity.yawRadPerSec, velocity.pitchRadPerSec) > 1e-4 ||
        Math.abs(velocity.zoomPerSec) > 1e-4,
    }
  }
  const safeDamping = Number.isFinite(damping) && damping >= 0 ? damping : 8
  const decay = Math.exp(-safeDamping * dtSec)
  // Integrate exponentially decaying velocity over [0, dtSec]
  const integratedDt = safeDamping > 1e-6 ? (1 - decay) / safeDamping : dtSec

  const nextYaw = state.yaw + velocity.yawRadPerSec * integratedDt
  const nextPitch = Math.max(
    -MAX_PITCH_RAD,
    Math.min(MAX_PITCH_RAD, state.pitch + velocity.pitchRadPerSec * integratedDt),
  )
  const zoomScale = Math.exp(velocity.zoomPerSec * integratedDt)
  const nextDist = Math.max(MIN_CAMERA_DIST, Math.min(MAX_CAMERA_DIST, state.dist * zoomScale))

  const nextVelocity: InertialOrbitVelocity = {
    yawRadPerSec: velocity.yawRadPerSec * decay,
    pitchRadPerSec: velocity.pitchRadPerSec * decay,
    zoomPerSec: velocity.zoomPerSec * decay,
  }
  const moving =
    Math.hypot(nextVelocity.yawRadPerSec, nextVelocity.pitchRadPerSec) > 1e-4 ||
    Math.abs(nextVelocity.zoomPerSec) > 1e-4
  if (!moving) {
    nextVelocity.yawRadPerSec = 0
    nextVelocity.pitchRadPerSec = 0
    nextVelocity.zoomPerSec = 0
  }

  return {
    state: {
      ...state,
      yaw: nextYaw,
      pitch: nextPitch,
      dist: nextDist,
    },
    velocity: nextVelocity,
    moving,
  }
}
