import { createHash } from 'node:crypto'

export const MAX_INDEPENDENT_ARTIFACT_CACHE_ENTRIES = 8
export const MAX_INDEPENDENT_ARTIFACT_CACHE_BYTES = 24 * 1024 * 1024

export type IndependentArtifactFormat = 'stl' | 'obj'

export interface IndependentArtifactInput {
  readonly format: IndependentArtifactFormat
  readonly fileName: string
  readonly mimeType: string
  readonly data: Uint8Array
}

export interface IndependentArtifactSummary {
  readonly id: string
  readonly format: IndependentArtifactFormat
  readonly fileName: string
  readonly mimeType: string
  readonly sha256: string
  readonly byteLength: number
}

export interface IndependentArtifact extends IndependentArtifactSummary {
  readonly data: Uint8Array
}

interface CachedArtifact extends IndependentArtifactSummary {
  readonly data: Uint8Array
}

export function independentArtifactMimeType(format: IndependentArtifactFormat): string {
  return format === 'stl' ? 'model/stl' : 'model/obj'
}

function dataSha256(data: Uint8Array): string {
  return createHash('sha256').update(data).digest('hex')
}

function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xD800 && unit <= 0xDBFF) {
      const next = value.charCodeAt(index + 1)
      if (next < 0xDC00 || next > 0xDFFF) return false
      index++
    } else if (unit >= 0xDC00 && unit <= 0xDFFF) {
      return false
    }
  }
  return true
}

function validateInput(input: IndependentArtifactInput, maxBytes: number): void {
  if (!(input.data instanceof Uint8Array)) {
    throw new TypeError('Independent artifact data must be a Uint8Array')
  }
  if (input.data.byteLength > maxBytes) {
    throw new RangeError(`Independent artifact exceeds cache capacity of ${maxBytes} bytes`)
  }
  const expectedMimeType = independentArtifactMimeType(input.format)
  if (input.mimeType !== expectedMimeType) {
    throw new TypeError(`Independent ${input.format.toUpperCase()} artifact must use ${expectedMimeType}`)
  }
  if (input.fileName.length < 1 || input.fileName.length > 255
    || !isWellFormedUnicode(input.fileName) || /[/\\\0]/.test(input.fileName)
    || !input.fileName.toLowerCase().endsWith(`.${input.format}`)) {
    throw new TypeError(`Independent ${input.format.toUpperCase()} artifact has an invalid file name`)
  }
}

/**
 * Per-MCP-session artifact storage for the repository-owned independent engine.
 *
 * The resource identifier is the SHA-256 of the exact exported bytes. These
 * artifacts deliberately bypass the legacy DuckDB build/artifact schema: that
 * schema carries a different engine provenance contract.
 */
export class IndependentArtifactCache {
  private readonly artifacts = new Map<string, CachedArtifact>()
  private totalBytes = 0

  constructor(
    private readonly maxEntries = MAX_INDEPENDENT_ARTIFACT_CACHE_ENTRIES,
    private readonly maxBytes = MAX_INDEPENDENT_ARTIFACT_CACHE_BYTES,
  ) {
    if (!Number.isSafeInteger(maxEntries) || maxEntries < 1) {
      throw new RangeError('Independent artifact cache maxEntries must be a positive integer')
    }
    if (!Number.isSafeInteger(maxBytes) || maxBytes < 1) {
      throw new RangeError('Independent artifact cache maxBytes must be a positive integer')
    }
  }

  put(input: IndependentArtifactInput): IndependentArtifactSummary {
    validateInput(input, this.maxBytes)
    const id = dataSha256(input.data)
    const existing = this.artifacts.get(id)
    if (existing && (existing.format !== input.format || existing.mimeType !== input.mimeType)) {
      throw new Error('Independent artifact content address has conflicting metadata')
    }
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
      sha256: id,
      byteLength: data.byteLength,
      data,
    })
    this.artifacts.set(id, artifact)
    this.totalBytes += artifact.byteLength
    this.evictToLimits()
    return this.toSummary(artifact)
  }

  get(id: string): IndependentArtifact | null {
    const artifact = this.artifacts.get(id)
    if (!artifact) return null
    this.artifacts.delete(id)
    this.artifacts.set(id, artifact)
    return Object.freeze({ ...this.toSummary(artifact), data: Uint8Array.from(artifact.data) })
  }

  list(): IndependentArtifactSummary[] {
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

  private toSummary(artifact: CachedArtifact): IndependentArtifactSummary {
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
