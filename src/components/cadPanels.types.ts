export type PanelLocale = 'ru' | 'en'

export type MeshKey = number | string

export type Vector3 = readonly [x: number, y: number, z: number]

export interface SourceProvenanceRow {
  id: MeshKey
  label?: string
  sourceStart: number
  sourceEnd: number
  line?: number
  column?: number
  originalId?: number
  triangleCount?: number
  color?: string | readonly [red: number, green: number, blue: number, alpha?: number]
}

export interface SceneMeshRow {
  id: MeshKey
  name: string
  visible: boolean
  locked?: boolean
  disabled?: boolean
  triangleCount: number
  color?: string | readonly [red: number, green: number, blue: number, alpha?: number]
  sources?: readonly SourceProvenanceRow[]
}

export interface InspectBounds {
  min: Vector3
  max: Vector3
}

export interface InspectSelection {
  meshId: MeshKey
  meshName: string
  triangleCount?: number
  surfaceArea?: number
  volume?: number
  position?: Vector3
  normal?: Vector3
  bounds?: InspectBounds
  source?: SourceProvenanceRow
}

export interface DistanceMeasurement {
  id?: MeshKey
  label?: string
  points: readonly Vector3[]
  distance?: number
}

export type SectionAxis = 'x' | 'y' | 'z'
