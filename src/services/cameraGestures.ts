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
