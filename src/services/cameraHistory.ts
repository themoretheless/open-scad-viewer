export type CameraProjection = 'perspective' | 'orthographic'

export interface CameraState {
  yaw: number
  pitch: number
  distance: number
  target: readonly [x: number, y: number, z: number]
  projection: CameraProjection
}

function finiteCameraState(state: CameraState) {
  return [state.yaw, state.pitch, state.distance, ...state.target].every(Number.isFinite)
    && state.distance > 0
    && (state.projection === 'perspective' || state.projection === 'orthographic')
}

function cloneCameraState(state: CameraState): CameraState {
  return {
    yaw: state.yaw,
    pitch: state.pitch,
    distance: state.distance,
    target: [...state.target],
    projection: state.projection,
  }
}

export function cameraStatesEqual(a: CameraState, b: CameraState, epsilon = 1e-8) {
  if (a.projection !== b.projection) return false
  const scale = Math.max(1, Math.abs(a.distance), Math.abs(b.distance))
  return Math.abs(a.yaw - b.yaw) <= epsilon
    && Math.abs(a.pitch - b.pitch) <= epsilon
    && Math.abs(a.distance - b.distance) <= epsilon * scale
    && a.target.every((value, axis) => Math.abs(value - b.target[axis]) <= epsilon * scale)
}

/** Bounded, deduplicated snapshots captured before committed camera actions. */
export class CameraHistory {
  private entries: CameraState[] = []

  constructor(readonly limit = 32) {
    if (!Number.isInteger(limit) || limit < 1) throw new RangeError('Camera history limit must be a positive integer')
  }

  get size() { return this.entries.length }

  record(state: CameraState): boolean {
    if (!finiteCameraState(state)) return false
    const last = this.entries[this.entries.length - 1]
    if (last && cameraStatesEqual(last, state)) return false
    this.entries.push(cloneCameraState(state))
    if (this.entries.length > this.limit) this.entries.splice(0, this.entries.length - this.limit)
    return true
  }

  previous(): CameraState | null {
    const state = this.entries.pop()
    return state ? cloneCameraState(state) : null
  }

  clear() { this.entries = [] }
}
