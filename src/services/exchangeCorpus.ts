import { ParserInputError, preflightIgesInput, preflightStlInput } from './parserInputBudgets'
import { buildStepRawArtifact } from './stepRawArtifact'

export type ExchangeCorpusFormat = 'step' | 'iges' | 'stl'
export type ExchangeCorpusOrigin = 'self-authored' | 'CAX-IF' | 'NIST'
export type ExchangeSourceAvailability = 'local' | 'downloaded' | 'not-downloaded' | 'unavailable'

export interface ExchangeCorpusFileResult {
  readonly path: string
  readonly sha256: string
  readonly format: ExchangeCorpusFormat
  readonly bytes: number
  readonly status: 'admitted' | 'refused'
  readonly entities?: number
  readonly faces?: number
  readonly controlPoints?: number
  readonly errorCode?: string
  readonly message?: string
}

export interface ExchangeCorpusSourceReport {
  readonly id: string
  readonly origin: ExchangeCorpusOrigin
  readonly locator: string
  readonly availability: ExchangeSourceAvailability
  readonly files: readonly ExchangeCorpusFileResult[]
}

export interface ExchangeCorpusReport {
  readonly version: 1
  readonly semantics: 'parser-admission-only'
  readonly note: string
  readonly sources: readonly ExchangeCorpusSourceReport[]
  readonly totals: {
    readonly sources: number
    readonly downloadedSources: number
    readonly unavailableSources: number
    readonly files: number
    readonly admitted: number
    readonly refused: number
    readonly bytes: number
    readonly entities: number
    readonly faces: number
    readonly controlPoints: number
  }
}

export function analyzeExchangeCorpusFile(
  path: string,
  format: ExchangeCorpusFormat,
  bytes: Uint8Array,
  sha256: string,
): ExchangeCorpusFileResult {
  try {
    const stats = format === 'step'
      ? buildStepRawArtifact(new TextDecoder().decode(bytes)).stats
      : format === 'iges'
        ? preflightIgesInput(new TextDecoder().decode(bytes))
        : preflightStlInput(bytes)
    return {
      path, sha256, format, bytes: bytes.byteLength, status: 'admitted',
      entities: stats.entities, faces: stats.faces, controlPoints: stats.controlPoints,
    }
  } catch (error) {
    return {
      path, sha256, format, bytes: bytes.byteLength, status: 'refused',
      errorCode: error instanceof ParserInputError ? error.code : 'invalid-input',
      message: error instanceof Error ? error.message : String(error),
    }
  }
}

export function createExchangeCorpusReport(sources: readonly ExchangeCorpusSourceReport[]): ExchangeCorpusReport {
  const files = sources.flatMap(source => source.files)
  const sum = (select: (file: ExchangeCorpusFileResult) => number | undefined) =>
    files.reduce((total, file) => total + (select(file) ?? 0), 0)
  return {
    version: 1,
    semantics: 'parser-admission-only',
    note: 'Admission statistics are not geometric conformance results. CAX-IF/NIST sources are counted only from local downloaded files; the existing NIST validator semantics are unchanged.',
    sources,
    totals: {
      sources: sources.length,
      downloadedSources: sources.filter(source => source.availability === 'local' || source.availability === 'downloaded').length,
      unavailableSources: sources.filter(source => source.availability === 'not-downloaded' || source.availability === 'unavailable').length,
      files: files.length,
      admitted: files.filter(file => file.status === 'admitted').length,
      refused: files.filter(file => file.status === 'refused').length,
      bytes: sum(file => file.bytes),
      entities: sum(file => file.entities),
      faces: sum(file => file.faces),
      controlPoints: sum(file => file.controlPoints),
    },
  }
}
