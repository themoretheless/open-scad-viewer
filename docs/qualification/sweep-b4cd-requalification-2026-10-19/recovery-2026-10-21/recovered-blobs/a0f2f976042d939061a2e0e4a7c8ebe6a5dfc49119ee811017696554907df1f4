import type { CameraState } from './cameraHistory'

export type ProjectionMode = 'perspective' | 'orthographic'
export type StandardView = 'iso' | 'front' | 'back' | 'left' | 'right' | 'top' | 'bottom'

export const ISO_YAW = Math.PI / 4
export const ISO_PITCH = Math.atan(1 / Math.sqrt(2))

const ORIENTATIONS: Readonly<Record<StandardView, readonly [yaw: number, pitch: number]>> = Object.freeze({
  iso: [ISO_YAW, ISO_PITCH],
  front: [0, 0],
  back: [Math.PI, 0],
  left: [-Math.PI / 2, 0],
  right: [Math.PI / 2, 0],
  top: [0, Math.PI / 2],
  bottom: [0, -Math.PI / 2],
})

export function standardViewOrientation(view: StandardView) {
  return ORIENTATIONS[view]
}

export function standardViewForCamera(state: Pick<CameraState, 'yaw' | 'pitch'>): StandardView | null {
  const angularDistance = (a: number, b: number) => Math.abs(Math.atan2(Math.sin(a - b), Math.cos(a - b)))
  for (const [view, [yaw, pitch]] of Object.entries(ORIENTATIONS) as Array<[StandardView, readonly [number, number]]>) {
    if (angularDistance(state.yaw, yaw) <= 1e-7 && Math.abs(state.pitch - pitch) <= 1e-7) return view
  }
  return null
}

export interface AxisScreenVector {
  readonly x: number
  readonly y: number
  readonly depth: number
}

export interface AxesScreenProjection {
  readonly x: AxisScreenVector
  readonly y: AxisScreenVector
  readonly z: AxisScreenVector
}

/** Project Z-up world axes into the renderer's orbit-camera screen basis. */
export function projectAxesToScreen(yaw: number, pitch: number): AxesScreenProjection {
  const cy = Math.cos(yaw), sy = Math.sin(yaw)
  const cp = Math.cos(pitch), sp = Math.sin(pitch)
  if (Math.abs(cp) < 1e-10) {
    const sign = sp >= 0 ? 1 : -1
    return {
      x: { x: sign, y: 0, depth: 0 },
      y: { x: 0, y: 1, depth: 0 },
      z: { x: 0, y: 0, depth: sign },
    }
  }
  return {
    x: { x: cy, y: -sp * sy, depth: cp * sy },
    y: { x: sy, y: sp * cy, depth: -cp * cy },
    z: { x: 0, y: cp, depth: sp },
  }
}
