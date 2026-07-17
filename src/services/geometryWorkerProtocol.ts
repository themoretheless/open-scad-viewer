import type { GeometryQuality, MeshData } from './openscadParser'

export interface GeometryRequest {
  id: number
  source: string
  quality: GeometryQuality
}

export interface GeometrySuccess {
  id: number
  ok: true
  meshes: MeshData[]
  warnings: string[]
  volume: number
  surfaceArea: number
  quality: GeometryQuality
  durationMs: number
}

export interface GeometryFailure {
  id: number
  ok: false
  error: {
    name: string
    message: string
    line?: number
    column?: number
  }
  durationMs: number
}

export type GeometryResponse = GeometrySuccess | GeometryFailure
