import {
  CameraHistory,
  cameraStatesEqual,
  type CameraHistorySnapshot,
  type CameraState,
} from './cameraHistory'
import {
  ISO_PITCH,
  ISO_YAW,
  standardViewForCamera,
  type ProjectionMode,
  type StandardView,
} from './viewportModel'

export interface ViewportState {
  readonly camera: CameraState
  readonly standardView: StandardView
  readonly activeView: StandardView | 'custom'
  readonly projectionBeforeFaceSnap: ProjectionMode | null
  readonly canGoBack: boolean
  readonly recoveryRevision: number
}

export interface ViewportNavigationSnapshot {
  readonly camera: CameraState
  readonly history: CameraHistorySnapshot
  readonly recoveryRevision: number
}

const FACE_VIEWS: readonly StandardView[] = ['front', 'back', 'left', 'right', 'top', 'bottom']

const DEFAULT_CAMERA: CameraState = Object.freeze({
  yaw: ISO_YAW,
  pitch: ISO_PITCH,
  distance: 180,
  target: Object.freeze([0, 0, 0] as const),
  projection: 'perspective',
})

function cloneCamera(state: CameraState): CameraState {
  return {
    yaw: state.yaw,
    pitch: state.pitch,
    distance: state.distance,
    target: [...state.target],
    projection: state.projection,
  }
}

type ViewportListener = (state: ViewportState) => void

/**
 * CPU owner for camera, view presets, history availability and recovery
 * generation. The renderer remains a disposable projection of this state.
 */
export class ViewportController {
  private readonly history: CameraHistory
  private snapshot: ViewportState
  private activeRecoveryToken: number | null = null
  private readonly listeners = new Set<ViewportListener>()

  constructor(historyLimit = 32) {
    this.history = new CameraHistory(historyLimit)
    this.snapshot = Object.freeze({
      camera: cloneCamera(DEFAULT_CAMERA),
      standardView: 'iso' as const,
      activeView: 'iso' as const,
      projectionBeforeFaceSnap: null,
      canGoBack: false,
      recoveryRevision: 0,
    })
  }

  get state(): ViewportState { return this.snapshot }

  get isRecovering(): boolean { return this.activeRecoveryToken !== null }

  subscribe(listener: ViewportListener): () => void {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  /** Mirror a renderer camera into the CPU owner and classify the standard view. */
  applyCamera(state: CameraState): ViewportState {
    const matched = standardViewForCamera(state)
    let projectionBeforeFaceSnap = this.snapshot.projectionBeforeFaceSnap
    let camera = cloneCamera(state)
    if (matched === null || matched === 'iso') {
      const restored = this.takeProjectionAfterFaceSnap(state.projection)
      projectionBeforeFaceSnap = null
      if (restored !== null && restored !== state.projection) {
        camera = { ...camera, projection: restored }
      }
    }
    return this.commit({
      camera,
      standardView: matched ?? this.snapshot.standardView,
      activeView: matched ?? 'custom',
      projectionBeforeFaceSnap,
    })
  }

  applyHistoryAvailability(canGoBack: boolean): ViewportState {
    return this.commit({ canGoBack })
  }

  applyRestoredCamera(state: CameraState): ViewportState {
    const matched = standardViewForCamera(state)
    return this.commit({
      camera: cloneCamera(state),
      standardView: matched ?? this.snapshot.standardView,
      activeView: matched ?? 'custom',
      projectionBeforeFaceSnap: null,
    })
  }

  resetView(): ViewportState {
    const restored = this.takeProjectionAfterFaceSnap(this.snapshot.camera.projection)
    return this.commit({
      camera: {
        ...cloneCamera(DEFAULT_CAMERA),
        projection: restored ?? this.snapshot.camera.projection,
      },
      standardView: 'iso',
      activeView: 'iso',
      projectionBeforeFaceSnap: null,
    })
  }

  toggleProjection(): ProjectionMode {
    const projection = this.snapshot.camera.projection === 'perspective' ? 'orthographic' : 'perspective'
    this.commit({
      camera: { ...this.snapshot.camera, projection },
      projectionBeforeFaceSnap: null,
    })
    return projection
  }

  setStandardView(view: StandardView): ViewportState {
    return this.commit({ standardView: view })
  }

  /**
   * Apply a named view. Face presets snap to orthographic and remember the
   * previous perspective projection as one logical action.
   */
  applyStandardView(view: StandardView): { view: StandardView; projection: ProjectionMode } {
    let projection = this.snapshot.camera.projection
    let projectionBeforeFaceSnap = this.snapshot.projectionBeforeFaceSnap
    if (FACE_VIEWS.includes(view)) {
      if (projectionBeforeFaceSnap === null && projection === 'perspective') {
        projectionBeforeFaceSnap = 'perspective'
      }
      projection = 'orthographic'
    } else {
      const previous = projectionBeforeFaceSnap
      projectionBeforeFaceSnap = null
      if (previous !== null) projection = previous
    }
    this.commit({
      camera: { ...this.snapshot.camera, projection },
      standardView: view,
      activeView: view,
      projectionBeforeFaceSnap,
    })
    return { view, projection }
  }

  recordHistory(state: CameraState): boolean {
    return this.history.record(state)
  }

  restoreHistory(snapshot: CameraHistorySnapshot): boolean {
    return this.history.restore(snapshot)
  }

  historySnapshot(): CameraHistorySnapshot {
    return this.history.snapshot()
  }

  beginRecovery(): number {
    const recoveryRevision = this.snapshot.recoveryRevision + 1
    this.activeRecoveryToken = recoveryRevision
    this.commit({ recoveryRevision })
    return recoveryRevision
  }

  isCurrentRecovery(token: number): boolean {
    return token === this.snapshot.recoveryRevision
  }

  invalidateRecovery(): void {
    this.activeRecoveryToken = null
    this.commit({ recoveryRevision: this.snapshot.recoveryRevision + 1 })
  }

  completeRecovery(token: number): void {
    if (this.activeRecoveryToken === token) this.activeRecoveryToken = null
  }

  navigationSnapshot(): ViewportNavigationSnapshot {
    return {
      camera: cloneCamera(this.snapshot.camera),
      history: this.history.snapshot(),
      recoveryRevision: this.snapshot.recoveryRevision,
    }
  }

  sameCamera(state: CameraState): boolean {
    return cameraStatesEqual(this.snapshot.camera, state)
  }

  private takeProjectionAfterFaceSnap(current: ProjectionMode): ProjectionMode | null {
    const previous = this.snapshot.projectionBeforeFaceSnap
    if (previous === null || previous === current) return null
    return previous
  }

  private commit(patch: Partial<ViewportState>): ViewportState {
    const next = Object.freeze({
      camera: patch.camera ? cloneCamera(patch.camera) : this.snapshot.camera,
      standardView: patch.standardView ?? this.snapshot.standardView,
      activeView: patch.activeView ?? this.snapshot.activeView,
      projectionBeforeFaceSnap: patch.projectionBeforeFaceSnap === undefined
        ? this.snapshot.projectionBeforeFaceSnap
        : patch.projectionBeforeFaceSnap,
      canGoBack: patch.canGoBack ?? this.snapshot.canGoBack,
      recoveryRevision: patch.recoveryRevision ?? this.snapshot.recoveryRevision,
    })
    if (
      next.standardView === this.snapshot.standardView
      && next.activeView === this.snapshot.activeView
      && next.projectionBeforeFaceSnap === this.snapshot.projectionBeforeFaceSnap
      && next.canGoBack === this.snapshot.canGoBack
      && next.recoveryRevision === this.snapshot.recoveryRevision
      && cameraStatesEqual(next.camera, this.snapshot.camera)
    ) return this.snapshot
    this.snapshot = next
    for (const listener of this.listeners) listener(next)
    return next
  }
}
