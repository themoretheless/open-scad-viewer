const FOV_Y = Math.PI / 4
const MIN_DISTANCE = 0.01
const MAX_DISTANCE = 1e12
const DEFAULT_DISTANCE = 50
const MAX_ORBIT_PITCH = Math.PI / 2 - 0.001

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

export function clampGestureDistance(distance: number) {
  return Math.max(MIN_DISTANCE, Math.min(MAX_DISTANCE, Number.isFinite(distance) ? distance : DEFAULT_DISTANCE))
}

export function computeOrbitUpdate(state: OrbitCameraState, dx: number, dy: number): OrbitCameraState {
  let yaw = state.yaw - dx * 0.005
  if (yaw > Math.PI || yaw < -Math.PI) yaw = ((yaw + Math.PI) % (2 * Math.PI) + 2 * Math.PI) % (2 * Math.PI) - Math.PI
  const pitch = Math.max(-MAX_ORBIT_PITCH, Math.min(MAX_ORBIT_PITCH, state.pitch + dy * 0.005))
  return { ...state, yaw, pitch }
}

export function computePanUpdate(state: OrbitCameraState, dx: number, dy: number, viewportHeight: number): OrbitCameraState {
  const scale = 2 * state.dist * Math.tan(FOV_Y / 2) / Math.max(1, viewportHeight)
  const cy = Math.cos(state.yaw), sy = Math.sin(state.yaw)
  const cp = Math.cos(state.pitch), sp = Math.sin(state.pitch)
  const rx = cy, ry = sy
  const ux = -sp * sy, uy = sp * cy, uz = cp
  return {
    ...state,
    tx: state.tx + (-dx * rx + dy * ux) * scale,
    ty: state.ty + (-dx * ry + dy * uy) * scale,
    tz: state.tz + dy * uz * scale,
  }
}

export function computeWheelDistance(distance: number, deltaPixels: number) {
  const delta = Math.max(-1000, Math.min(1000, deltaPixels))
  return clampGestureDistance(distance * Math.exp(delta * 0.001))
}

export function computePinchUpdate(
  prevA: PinchPoint,
  prevB: PinchPoint,
  curA: PinchPoint,
  curB: PinchPoint,
  state: PinchCameraState,
  viewportHeight: number,
): PinchCameraState {
  const prevSpan = Math.hypot(prevB.x - prevA.x, prevB.y - prevA.y)
  const curSpan = Math.hypot(curB.x - curA.x, curB.y - curA.y)
  let dist = state.dist
  if (prevSpan > 1e-6 && curSpan > 1e-6) {
    const ratio = Math.max(Math.exp(-1), Math.min(Math.exp(1), prevSpan / curSpan))
    dist = clampGestureDistance(state.dist * ratio)
  }
  const moved = computePanUpdate({ ...state, dist },
    (curA.x + curB.x - prevA.x - prevB.x) / 2,
    (curA.y + curB.y - prevA.y - prevB.y) / 2,
    viewportHeight)
  return moved
}
