import type { GeometryPhaseTimings, GeometryQuality } from '../core/build'
import type { SceneUploadMetrics } from './rendererContracts'

export interface BuildPerformanceSample {
  token: number
  revision: number
  quality: GeometryQuality
  phases: GeometryPhaseTimings
  workerMs: number
  hostMs: number | null
  publicationMs: number
  editToSubmitMs: number | null
  publicationToSubmitMs: number | null
  transferBytes: number
  upload: SceneUploadMetrics | null
}

/** Bounded, source-free session diagnostics. All timestamps use the main clock. */
export class BuildPerformanceHistory {
  private samples: BuildPerformanceSample[] = []
  private pending = new Map<number, { publishedAt: number; editedAt: number | null }>()
  private edit: { revision: number; at: number } | null = null
  private nextToken = 1

  edited(revision: number, at: number) { this.edit = { revision, at } }

  record(sample: Omit<BuildPerformanceSample, 'token' | 'editToSubmitMs' | 'publicationToSubmitMs'>, publishedAt: number): number {
    const token = this.nextToken++
    this.samples.push({ ...sample, phases: { ...sample.phases }, upload: sample.upload && { ...sample.upload }, token, editToSubmitMs: null, publicationToSubmitMs: null })
    // Only the latest published scene can reach its first submission now.
    this.pending.clear()
    this.pending.set(token, { publishedAt, editedAt: this.edit?.revision === sample.revision ? this.edit.at : null })
    if (this.samples.length > 60) this.samples.shift()
    return token
  }

  submitted(token: number, at: number): boolean {
    const pending = this.pending.get(token)
    if (!pending || at < pending.publishedAt) return false
    const sample = this.samples.find(candidate => candidate.token === token)
    this.pending.delete(token)
    if (!sample) return false
    sample.publicationToSubmitMs = at - pending.publishedAt
    sample.editToSubmitMs = pending.editedAt === null ? null : Math.max(0, at - pending.editedAt)
    return true
  }

  snapshot(): BuildPerformanceSample[] {
    return this.samples.map(sample => ({ ...sample, phases: { ...sample.phases }, upload: sample.upload && { ...sample.upload } }))
  }
}
