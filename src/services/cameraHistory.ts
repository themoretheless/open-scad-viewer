export type CameraProjection = 'perspective' | 'orthographic'

export interface CameraState {
  yaw: number
  pitch: number
  distance: number
  target: readonly [x: number, y: number, z: number]
  projection: CameraProjection
}

export type CameraHistorySnapshot = readonly CameraState[]

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
  private redoStack: CameraState[] = []

  constructor(readonly limit = 32, readonly epsilon = 1e-8) {
    if (!Number.isInteger(limit) || limit < 1) throw new RangeError('Camera history limit must be a positive integer')
    if (!Number.isFinite(epsilon) || epsilon < 0) throw new RangeError('Camera history epsilon must be a non-negative finite number')
  }

  get size() { return this.entries.length }

  get redoSize() { return this.redoStack.length }

  canUndo(): boolean { return this.entries.length > 0 }

  canRedo(): boolean { return this.redoStack.length > 0 }

  snapshot(): CameraState[] {
    return this.entries.map(cloneCameraState)
  }

  /** Atomically replace history from an immutable recovery snapshot. */
  restore(snapshot: CameraHistorySnapshot): boolean {
    if (!Array.isArray(snapshot) || snapshot.some(state => !finiteCameraState(state))) return false
    const restored = new CameraHistory(this.limit, this.epsilon)
    for (const state of snapshot) restored.record(state)
    this.entries = restored.entries
    this.redoStack = []
    return true
  }

  record(state: CameraState): boolean {
    if (!finiteCameraState(state)) return false
    const last = this.entries[this.entries.length - 1]
    if (last && cameraStatesEqual(last, state, this.epsilon)) return false
    this.entries.push(cloneCameraState(state))
    if (this.entries.length > this.limit) this.entries.splice(0, this.entries.length - this.limit)
    this.redoStack = []
    return true
  }

  previous(): CameraState | null {
    const state = this.entries.pop()
    if (!state) return null
    const cloned = cloneCameraState(state)
    const lastRedo = this.redoStack[this.redoStack.length - 1]
    if (!lastRedo || !cameraStatesEqual(lastRedo, cloned, this.epsilon)) {
      this.redoStack.push(cloneCameraState(cloned))
      if (this.redoStack.length > this.limit) this.redoStack.splice(0, this.redoStack.length - this.limit)
    }
    return cloned
  }

  undo(): CameraState | null {
    return this.previous()
  }

  redo(): CameraState | null {
    const state = this.redoStack.pop()
    if (!state) return null
    const cloned = cloneCameraState(state)
    const last = this.entries[this.entries.length - 1]
    if (!last || !cameraStatesEqual(last, cloned, this.epsilon)) {
      this.entries.push(cloneCameraState(cloned))
      if (this.entries.length > this.limit) this.entries.splice(0, this.entries.length - this.limit)
    }
    return cloned
  }

  clear() {
    this.entries = []
    this.redoStack = []
  }
}
