import { createHash } from 'node:crypto'

export const MAX_OFFICIAL_ARTIFACT_CACHE_ENTRIES = 8
export const MAX_OFFICIAL_ARTIFACT_CACHE_BYTES = 24 * 1024 * 1024

export interface OfficialArtifactInput {
  readonly format: string
  readonly fileName: string
  readonly mimeType: string
  readonly data: Uint8Array
}

export interface OfficialArtifactSummary {
  readonly id: string
  readonly format: string
  readonly fileName: string
  readonly mimeType: string
  readonly sha256: string
  readonly byteLength: number
}

export interface OfficialArtifact extends OfficialArtifactSummary {
  readonly data: Uint8Array
}

interface CachedArtifact extends OfficialArtifactSummary {
  readonly data: Uint8Array
}

function artifactId(input: OfficialArtifactInput): string {
  return createHash('sha256')
    .update(input.format)
    .update('\0')
    .update(input.mimeType)
    .update('\0')
    .update(input.data)
    .digest('hex')
}

function dataSha256(data: Uint8Array): string {
  return createHash('sha256').update(data).digest('hex')
}

/**
 * Per-MCP-session, content-addressed cache for official-runtime exports.
 *
 * Official artifacts deliberately do not enter the legacy DuckDB build schema:
 * that schema attests the independent Manifold/B-rep router. The bounded cache
 * gives MCP clients a resource link without inventing legacy engine provenance.
 */
export class OfficialArtifactCache {
  private readonly artifacts = new Map<string, CachedArtifact>()
  private totalBytes = 0

  constructor(
    private readonly maxEntries = MAX_OFFICIAL_ARTIFACT_CACHE_ENTRIES,
    private readonly maxBytes = MAX_OFFICIAL_ARTIFACT_CACHE_BYTES,
  ) {
    if (!Number.isSafeInteger(maxEntries) || maxEntries < 1) {
      throw new RangeError('Official artifact cache maxEntries must be a positive integer')
    }
    if (!Number.isSafeInteger(maxBytes) || maxBytes < 1) {
      throw new RangeError('Official artifact cache maxBytes must be a positive integer')
    }
  }

  put(input: OfficialArtifactInput): OfficialArtifactSummary {
    if (!(input.data instanceof Uint8Array)) {
      throw new TypeError('Official artifact data must be a Uint8Array')
    }
    if (input.data.byteLength > this.maxBytes) {
      throw new RangeError(`Official artifact exceeds cache capacity of ${this.maxBytes} bytes`)
    }
    const id = artifactId(input)
    const existing = this.artifacts.get(id)
    if (existing) {
      this.artifacts.delete(id)
      this.totalBytes -= existing.byteLength
    }
    const data = Uint8Array.from(input.data)
    const artifact: CachedArtifact = Object.freeze({
      id,
      format: input.format,
      fileName: input.fileName,
      mimeType: input.mimeType,
      sha256: dataSha256(data),
      byteLength: data.byteLength,
      data,
    })
    this.artifacts.set(id, artifact)
    this.totalBytes += artifact.byteLength
    this.evictToLimits()
    return this.toSummary(artifact)
  }

  get(id: string): OfficialArtifact | null {
    const artifact = this.artifacts.get(id)
    if (!artifact) return null
    // Reading also refreshes LRU order without changing the content address.
    this.artifacts.delete(id)
    this.artifacts.set(id, artifact)
    return Object.freeze({ ...this.toSummary(artifact), data: Uint8Array.from(artifact.data) })
  }

  list(): OfficialArtifactSummary[] {
    return [...this.artifacts.values()].reverse().map(artifact => this.toSummary(artifact))
  }

  private evictToLimits(): void {
    while (this.artifacts.size > this.maxEntries || this.totalBytes > this.maxBytes) {
      const oldestId = this.artifacts.keys().next().value as string | undefined
      if (oldestId === undefined) break
      const oldest = this.artifacts.get(oldestId)!
      this.artifacts.delete(oldestId)
      this.totalBytes -= oldest.byteLength
    }
  }

  private toSummary(artifact: CachedArtifact): OfficialArtifactSummary {
    return Object.freeze({
      id: artifact.id,
      format: artifact.format,
      fileName: artifact.fileName,
      mimeType: artifact.mimeType,
      sha256: artifact.sha256,
      byteLength: artifact.byteLength,
    })
  }
}
