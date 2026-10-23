import type { MeshData } from '../core/mesh'
import type { DistanceMeasurement } from './rendererContracts'

export type SectionAxis = 'x' | 'y' | 'z'

export interface SectionState {
  readonly enabled: boolean
  readonly axis: SectionAxis
  readonly offset: number
  readonly flip: boolean
  readonly initialized: boolean
}

export interface SceneState<THit = unknown> {
  readonly meshes: MeshData[]
  readonly visibility: boolean[]
  readonly selectedIndex: number | null
  readonly selectedHit: THit | null
  readonly isolated: boolean
  readonly hoveredHit: THit | null
  readonly measurement: DistanceMeasurement | null
  readonly measureActive: boolean
  readonly section: SectionState
}

export interface SceneStatePatch<THit> {
  meshes?: readonly MeshData[]
  visibility?: readonly boolean[]
  selectedIndex?: number | null
  selectedHit?: THit | null
  isolated?: boolean
  hoveredHit?: THit | null
  measurement?: DistanceMeasurement | null
  measureActive?: boolean
  section?: Partial<SectionState>
}

type SceneStateListener<THit> = (state: SceneState<THit>) => void

const DEFAULT_SECTION: SectionState = Object.freeze({
  enabled: false,
  axis: 'z',
  offset: 0,
  flip: false,
  initialized: false,
})

function visibleHit<THit extends { meshIndex?: number } | unknown>(
  hit: THit | null,
  visibility: readonly boolean[],
): THit | null {
  if (hit === null || typeof hit !== 'object' || !('meshIndex' in hit)) return hit
  const index = (hit as { meshIndex?: unknown }).meshIndex
  if (typeof index !== 'number' || !Number.isInteger(index) || index < 0 || index >= visibility.length) return null
  return visibility[index] === false ? null : hit
}

/** Canonical CPU-side owner for scene identity and interaction state. */
export class SceneController<THit = unknown> {
  private snapshot: SceneState<THit> = Object.freeze({
    meshes: [],
    visibility: [],
    selectedIndex: null,
    selectedHit: null,
    isolated: false,
    hoveredHit: null,
    measurement: null,
    measureActive: false,
    section: DEFAULT_SECTION,
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
    const selectedHit = selectedIndex === null ? null : visibleHit(requestedHit, visibility)
    const requestedIsolation = patch.isolated === undefined ? this.snapshot.isolated : patch.isolated
    const isolated = selectedIndex !== null && requestedIsolation
    const requestedHover = patch.hoveredHit === undefined ? this.snapshot.hoveredHit : patch.hoveredHit
    const hoveredHit = visibleHit(requestedHover, visibility)
    const measurement = patch.measurement === undefined ? this.snapshot.measurement : patch.measurement
    const measureActive = patch.measureActive === undefined ? this.snapshot.measureActive : patch.measureActive
    const section = Object.freeze({
      ...this.snapshot.section,
      ...(patch.section ?? {}),
    })
    const next = Object.freeze({
      meshes,
      visibility,
      selectedIndex,
      selectedHit,
      isolated,
      hoveredHit,
      measurement,
      measureActive,
      section,
    })
    if (this.sameState(next)) return this.snapshot
    this.snapshot = next
    for (const listener of this.listeners) listener(next)
    return next
  }

  publish(patch: Required<Pick<SceneStatePatch<THit>, 'meshes' | 'visibility' | 'selectedIndex' | 'isolated'>>): SceneState<THit> {
    return this.update({ ...patch, selectedHit: null, hoveredHit: null })
  }

  applyRendererSelection(selectedIndex: number | null, isolated: boolean, selectedHit: THit | null): SceneState<THit> {
    return this.update({ selectedIndex, isolated, selectedHit })
  }

  applyRendererHover(hoveredHit: THit | null): SceneState<THit> {
    return this.update({ hoveredHit })
  }

  applyRendererMeasurement(measurement: DistanceMeasurement | null, measureActive: boolean): SceneState<THit> {
    return this.update({ measurement, measureActive })
  }

  setMeasurement(measurement: DistanceMeasurement | null, measureActive = false): SceneState<THit> {
    return this.update({ measurement, measureActive })
  }

  setSection(section: Partial<SectionState>): SceneState<THit> {
    return this.update({ section })
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
      && candidate.hoveredHit === this.snapshot.hoveredHit
      && candidate.measurement === this.snapshot.measurement
      && candidate.measureActive === this.snapshot.measureActive
      && candidate.section.enabled === this.snapshot.section.enabled
      && candidate.section.axis === this.snapshot.section.axis
      && candidate.section.offset === this.snapshot.section.offset
      && candidate.section.flip === this.snapshot.section.flip
      && candidate.section.initialized === this.snapshot.section.initialized
  }
}
