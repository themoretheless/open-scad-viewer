import type { MeshData } from '../core/mesh'

export interface SceneState<THit = unknown> {
  readonly meshes: MeshData[]
  readonly visibility: boolean[]
  readonly selectedIndex: number | null
  readonly selectedHit: THit | null
  readonly isolated: boolean
}

export interface SceneStatePatch<THit> {
  meshes?: readonly MeshData[]
  visibility?: readonly boolean[]
  selectedIndex?: number | null
  selectedHit?: THit | null
  isolated?: boolean
}

type SceneStateListener<THit> = (state: SceneState<THit>) => void

/** Canonical CPU-side owner for scene identity and interaction state. */
export class SceneController<THit = unknown> {
  private snapshot: SceneState<THit> = Object.freeze({
    meshes: [], visibility: [], selectedIndex: null, selectedHit: null, isolated: false,
  })
  private readonly listeners = new Set<SceneStateListener<THit>>()

  get state(): SceneState<THit> { return this.snapshot }

  subscribe(listener: SceneStateListener<THit>): () => void {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  update(patch: SceneStatePatch<THit>): SceneState<THit> {
    const meshes = patch.meshes === undefined ? this.snapshot.meshes : [...patch.meshes]
    const requestedVisibility = patch.visibility === undefined ? this.snapshot.visibility : [...patch.visibility]
    const visibility = Array.from({ length: meshes.length }, (_, index) => requestedVisibility[index] !== false)
    const candidate = patch.selectedIndex === undefined ? this.snapshot.selectedIndex : patch.selectedIndex
    const selectedIndex = candidate !== null
      && Number.isInteger(candidate)
      && candidate >= 0
      && candidate < meshes.length
      && visibility[candidate] !== false
      ? candidate
      : null
    const requestedHit = patch.selectedHit === undefined ? this.snapshot.selectedHit : patch.selectedHit
    const selectedHit = selectedIndex === null ? null : requestedHit
    const requestedIsolation = patch.isolated === undefined ? this.snapshot.isolated : patch.isolated
    const isolated = selectedIndex !== null && requestedIsolation
    const next = Object.freeze({ meshes, visibility, selectedIndex, selectedHit, isolated })
    if (this.sameState(next)) return this.snapshot
    this.snapshot = next
    for (const listener of this.listeners) listener(next)
    return next
  }

  publish(patch: Required<Pick<SceneStatePatch<THit>, 'meshes' | 'visibility' | 'selectedIndex' | 'isolated'>>): SceneState<THit> {
    return this.update({ ...patch, selectedHit: null })
  }

  applyRendererSelection(selectedIndex: number | null, isolated: boolean, selectedHit: THit | null): SceneState<THit> {
    return this.update({ selectedIndex, isolated, selectedHit })
  }

  setVisibility(index: number, visible: boolean): SceneState<THit> {
    if (!Number.isInteger(index) || index < 0 || index >= this.snapshot.meshes.length) return this.snapshot
    const visibility = this.snapshot.visibility.map((value, candidate) => candidate === index ? visible : value)
    return this.update({ visibility })
  }

  private sameState(candidate: SceneState<THit>) {
    return candidate.meshes.length === this.snapshot.meshes.length
      && candidate.meshes.every((mesh, index) => mesh === this.snapshot.meshes[index])
      && candidate.visibility.every((visible, index) => visible === this.snapshot.visibility[index])
      && candidate.selectedIndex === this.snapshot.selectedIndex
      && candidate.selectedHit === this.snapshot.selectedHit
      && candidate.isolated === this.snapshot.isolated
  }
}
