import { sha256Hex } from '../core/sha256'

export const OPENSCAD_PROJECT_SCHEMA_VERSION = 2 as const
export const OPENSCAD_PROJECT_MAX_FILES = 128
export const OPENSCAD_PROJECT_MAX_SOURCE_BYTES = 1_048_576
export const OPENSCAD_PROJECT_MAX_BLOB_BYTES = 4 * 1024 * 1024
export const OPENSCAD_PROJECT_MAX_TOTAL_BYTES = 6 * 1024 * 1024
export const OPENSCAD_PROJECT_MAX_PATH_BYTES = 1_024
export const OPENSCAD_PROJECT_MAX_SEGMENT_BYTES = 255

export const OPENSCAD_PROJECT_ERROR_CODES = Object.freeze([
  'E_PROJECT_INVALID_BUNDLE',
  'E_PROJECT_INVALID_PATH',
  'E_PROJECT_PATH_ESCAPE',
  'E_PROJECT_DUPLICATE_PATH',
  'E_PROJECT_FILE_COUNT_LIMIT',
  'E_PROJECT_INVALID_SOURCE',
  'E_PROJECT_INVALID_BLOB',
  'E_PROJECT_SOURCE_BYTES_LIMIT',
  'E_PROJECT_BLOB_BYTES_LIMIT',
  'E_PROJECT_TOTAL_BYTES_LIMIT',
  'E_PROJECT_ENTRYPOINT_MISSING',
  'E_PROJECT_ENTRYPOINT_NOT_SOURCE',
] as const)

export type OpenScadProjectErrorCode = typeof OPENSCAD_PROJECT_ERROR_CODES[number]

export interface OpenScadProjectErrorDetails {
  readonly path?: string
  readonly limit?: number
  readonly actual?: number
}

export class OpenScadProjectError extends Error {
  readonly details: Readonly<OpenScadProjectErrorDetails>

  constructor(
    readonly code: OpenScadProjectErrorCode,
    message: string,
    details: OpenScadProjectErrorDetails = {},
  ) {
    super(message)
    this.name = 'OpenScadProjectError'
    this.details = Object.freeze({ ...details })
  }
}

export interface OpenScadProjectSourceInput {
  readonly kind: 'source'
  readonly path: string
  readonly source: string
}

export interface OpenScadProjectBlobInput {
  readonly kind: 'blob'
  readonly path: string
  readonly data: Uint8Array
}

export type OpenScadProjectFileInput = OpenScadProjectSourceInput | OpenScadProjectBlobInput

export interface OpenScadProjectInput {
  readonly entrypoint: string
  readonly files: readonly OpenScadProjectFileInput[]
}

interface StoredSourceFile {
  readonly kind: 'source'
  readonly path: string
  readonly source: string
  readonly byteLength: number
  readonly sha256: string
}

interface StoredBlobFile {
  readonly kind: 'blob'
  readonly path: string
  readonly data: Uint8Array
  readonly byteLength: number
  readonly sha256: string
}

type StoredProjectFile = StoredSourceFile | StoredBlobFile

export interface OpenScadProjectSourceFile {
  readonly kind: 'source'
  readonly path: string
  readonly source: string
  readonly byteLength: number
  readonly sha256: string
}

export interface OpenScadProjectBlobFile {
  readonly kind: 'blob'
  readonly path: string
  /** A detached copy. Mutating it cannot change the project. */
  readonly data: Uint8Array
  readonly byteLength: number
  readonly sha256: string
}

export type OpenScadProjectFile = OpenScadProjectSourceFile | OpenScadProjectBlobFile

export interface OpenScadProjectFileSummary {
  readonly kind: OpenScadProjectFile['kind']
  readonly path: string
  readonly byteLength: number
  readonly sha256: string
}

export interface OpenScadProjectBundleV2 {
  readonly schemaVersion: typeof OPENSCAD_PROJECT_SCHEMA_VERSION
  readonly entrypoint: string
  readonly files: readonly OpenScadProjectFile[]
  readonly fileCount: number
  readonly byteLength: number
  readonly sha256: string
}

const UTF8 = new TextEncoder()

function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
}

function utf8Length(value: string): number {
  return UTF8.encode(value).byteLength
}

function invalidPath(message: string, path?: string): never {
  throw new OpenScadProjectError(
    'E_PROJECT_INVALID_PATH',
    message,
    path === undefined ? {} : { path },
  )
}

/** Canonical NFC, project-relative POSIX file path. */
export function normalizeOpenScadProjectPath(value: string): string {
  if (typeof value !== 'string' || value.length === 0 || !isWellFormedUnicode(value)) {
    return invalidPath('OpenSCAD project paths must be non-empty, well-formed Unicode strings.')
  }
  const path = value.normalize('NFC')
  const pathBytes = utf8Length(path)
  if (pathBytes > OPENSCAD_PROJECT_MAX_PATH_BYTES) {
    throw new OpenScadProjectError(
      'E_PROJECT_INVALID_PATH',
      `OpenSCAD project path exceeds ${OPENSCAD_PROJECT_MAX_PATH_BYTES} UTF-8 bytes.`,
      { path, limit: OPENSCAD_PROJECT_MAX_PATH_BYTES, actual: pathBytes },
    )
  }
  if (path.startsWith('/') || path.endsWith('/') || path.includes('\\')
    || /^[A-Za-z][A-Za-z0-9+.-]*:/u.test(path)) {
    return invalidPath('OpenSCAD project paths must be project-relative POSIX file paths.', path)
  }
  const segments = path.split('/')
  for (const segment of segments) {
    if (segment.length === 0 || segment === '.' || segment === '..'
      || /[\u0000-\u001f\u007f]/u.test(segment)) {
      return invalidPath('OpenSCAD project paths cannot contain empty, dot, parent, or control-character segments.', path)
    }
    const segmentBytes = utf8Length(segment)
    if (segmentBytes > OPENSCAD_PROJECT_MAX_SEGMENT_BYTES) {
      throw new OpenScadProjectError(
        'E_PROJECT_INVALID_PATH',
        `OpenSCAD project path segment exceeds ${OPENSCAD_PROJECT_MAX_SEGMENT_BYTES} UTF-8 bytes.`,
        { path, limit: OPENSCAD_PROJECT_MAX_SEGMENT_BYTES, actual: segmentBytes },
      )
    }
  }
  return segments.join('/')
}

/** Resolve a module-relative dependency without permitting traversal above the bundle root. */
export function resolveOpenScadProjectPath(importer: string, specifier: string): string {
  const importerPath = normalizeOpenScadProjectPath(importer)
  if (typeof specifier !== 'string' || specifier.length === 0
    || !isWellFormedUnicode(specifier)) {
    return invalidPath('OpenSCAD dependency paths must be non-empty, well-formed Unicode strings.')
  }
  const dependency = specifier.normalize('NFC')
  if (utf8Length(dependency) > OPENSCAD_PROJECT_MAX_PATH_BYTES
    || dependency.startsWith('/') || dependency.endsWith('/') || dependency.includes('\\')
    || dependency.includes('//')
    || /^[A-Za-z][A-Za-z0-9+.-]*:/u.test(dependency)
    || /[\u0000-\u001f\u007f]/u.test(dependency)) {
    return invalidPath('OpenSCAD dependencies must be bounded, relative POSIX file paths.', dependency)
  }
  const dependencySegments = dependency.split('/')
  if (dependencySegments.at(-1) === '.' || dependencySegments.at(-1) === '..') {
    return invalidPath('OpenSCAD dependency path must name a file.', dependency)
  }

  const resolved = importerPath.split('/')
  resolved.pop()
  for (const segment of dependencySegments) {
    if (segment.length === 0 || segment === '.') continue
    if (segment === '..') {
      if (resolved.length === 0) {
        throw new OpenScadProjectError(
          'E_PROJECT_PATH_ESCAPE',
          'OpenSCAD dependency path escapes the project root.',
          { path: dependency },
        )
      }
      resolved.pop()
      continue
    }
    resolved.push(segment)
  }
  if (resolved.length === 0) {
    return invalidPath('OpenSCAD dependency path must resolve to a file.', dependency)
  }
  return normalizeOpenScadProjectPath(resolved.join('/'))
}

function cloneFile(file: StoredProjectFile): OpenScadProjectFile {
  if (file.kind === 'source') return Object.freeze({ ...file })
  return Object.freeze({
    kind: file.kind,
    path: file.path,
    data: new Uint8Array(file.data),
    byteLength: file.byteLength,
    sha256: file.sha256,
  })
}

function canonicalProjectDigest(
  entrypoint: string,
  files: readonly OpenScadProjectFileSummary[],
): string {
  return sha256Hex(JSON.stringify({
    schemaVersion: OPENSCAD_PROJECT_SCHEMA_VERSION,
    entrypoint,
    files: files.map(file => ({
      kind: file.kind,
      path: file.path,
      byteLength: file.byteLength,
      sha256: file.sha256,
    })),
  }))
}

/**
 * Immutable, binary-safe project snapshot for the independent language engine.
 * Binary inputs are copied on admission and every binary read returns a new copy.
 */
export class OpenScadProject {
  readonly schemaVersion = OPENSCAD_PROJECT_SCHEMA_VERSION
  readonly entrypoint: string
  readonly fileCount: number
  readonly byteLength: number
  readonly sha256: string

  readonly #files: ReadonlyMap<string, StoredProjectFile>
  readonly #summaries: readonly OpenScadProjectFileSummary[]

  constructor(input: OpenScadProjectInput) {
    if (!input || typeof input !== 'object' || !Array.isArray(input.files)) {
      throw new OpenScadProjectError(
        'E_PROJECT_INVALID_BUNDLE',
        'OpenSCAD project input must contain an entrypoint and a file array.',
      )
    }
    if (input.files.length > OPENSCAD_PROJECT_MAX_FILES) {
      throw new OpenScadProjectError(
        'E_PROJECT_FILE_COUNT_LIMIT',
        `OpenSCAD project exceeds ${OPENSCAD_PROJECT_MAX_FILES} files.`,
        { limit: OPENSCAD_PROJECT_MAX_FILES, actual: input.files.length },
      )
    }

    const entrypoint = normalizeOpenScadProjectPath(input.entrypoint)
    const files = new Map<string, StoredProjectFile>()
    let totalBytes = 0
    for (const inputFile of input.files) {
      if (!inputFile || typeof inputFile !== 'object'
        || (inputFile.kind !== 'source' && inputFile.kind !== 'blob')) {
        throw new OpenScadProjectError(
          'E_PROJECT_INVALID_BUNDLE',
          'OpenSCAD project files must be tagged source or blob values.',
        )
      }
      const path = normalizeOpenScadProjectPath(inputFile.path)
      if (files.has(path)) {
        throw new OpenScadProjectError(
          'E_PROJECT_DUPLICATE_PATH',
          `OpenSCAD project contains duplicate path ${path}.`,
          { path },
        )
      }

      let stored: StoredProjectFile
      if (inputFile.kind === 'source') {
        if (typeof inputFile.source !== 'string' || !isWellFormedUnicode(inputFile.source)) {
          throw new OpenScadProjectError(
            'E_PROJECT_INVALID_SOURCE',
            `OpenSCAD source file ${path} must contain well-formed Unicode.`,
            { path },
          )
        }
        const bytes = UTF8.encode(inputFile.source)
        if (bytes.byteLength > OPENSCAD_PROJECT_MAX_SOURCE_BYTES) {
          throw new OpenScadProjectError(
            'E_PROJECT_SOURCE_BYTES_LIMIT',
            `OpenSCAD source file ${path} exceeds ${OPENSCAD_PROJECT_MAX_SOURCE_BYTES} UTF-8 bytes.`,
            { path, limit: OPENSCAD_PROJECT_MAX_SOURCE_BYTES, actual: bytes.byteLength },
          )
        }
        stored = Object.freeze({
          kind: 'source',
          path,
          source: inputFile.source,
          byteLength: bytes.byteLength,
          sha256: sha256Hex(bytes),
        })
      } else {
        if (!(inputFile.data instanceof Uint8Array)) {
          throw new OpenScadProjectError(
            'E_PROJECT_INVALID_BLOB',
            `OpenSCAD blob file ${path} must contain Uint8Array data.`,
            { path },
          )
        }
        const data = new Uint8Array(inputFile.data)
        if (data.byteLength > OPENSCAD_PROJECT_MAX_BLOB_BYTES) {
          throw new OpenScadProjectError(
            'E_PROJECT_BLOB_BYTES_LIMIT',
            `OpenSCAD blob file ${path} exceeds ${OPENSCAD_PROJECT_MAX_BLOB_BYTES} bytes.`,
            { path, limit: OPENSCAD_PROJECT_MAX_BLOB_BYTES, actual: data.byteLength },
          )
        }
        stored = Object.freeze({
          kind: 'blob',
          path,
          data,
          byteLength: data.byteLength,
          sha256: sha256Hex(data),
        })
      }

      totalBytes += stored.byteLength
      if (totalBytes > OPENSCAD_PROJECT_MAX_TOTAL_BYTES) {
        throw new OpenScadProjectError(
          'E_PROJECT_TOTAL_BYTES_LIMIT',
          `OpenSCAD project exceeds ${OPENSCAD_PROJECT_MAX_TOTAL_BYTES} bytes.`,
          { limit: OPENSCAD_PROJECT_MAX_TOTAL_BYTES, actual: totalBytes },
        )
      }
      files.set(path, stored)
    }

    const entrypointFile = files.get(entrypoint)
    if (entrypointFile === undefined) {
      throw new OpenScadProjectError(
        'E_PROJECT_ENTRYPOINT_MISSING',
        `OpenSCAD project entrypoint ${entrypoint} is missing.`,
        { path: entrypoint },
      )
    }
    if (entrypointFile.kind !== 'source') {
      throw new OpenScadProjectError(
        'E_PROJECT_ENTRYPOINT_NOT_SOURCE',
        `OpenSCAD project entrypoint ${entrypoint} must be a source file.`,
        { path: entrypoint },
      )
    }

    const summaries = [...files.values()]
      .map(file => Object.freeze({
        kind: file.kind,
        path: file.path,
        byteLength: file.byteLength,
        sha256: file.sha256,
      }))
      .sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0)

    this.entrypoint = entrypoint
    this.fileCount = files.size
    this.byteLength = totalBytes
    this.#files = files
    this.#summaries = Object.freeze(summaries)
    this.sha256 = canonicalProjectDigest(entrypoint, summaries)
    Object.freeze(this)
  }

  read(path: string): OpenScadProjectFile | null {
    const file = this.#files.get(normalizeOpenScadProjectPath(path))
    return file === undefined ? null : cloneFile(file)
  }

  readEntrypoint(): OpenScadProjectSourceFile {
    return this.read(this.entrypoint) as OpenScadProjectSourceFile
  }

  resolve(importer: string, specifier: string): OpenScadProjectFile | null {
    return this.read(resolveOpenScadProjectPath(importer, specifier))
  }

  list(): readonly OpenScadProjectFileSummary[] {
    return Object.freeze([...this.#summaries])
  }

  toBundle(): OpenScadProjectBundleV2 {
    return Object.freeze({
      schemaVersion: this.schemaVersion,
      entrypoint: this.entrypoint,
      files: Object.freeze(this.#summaries.map(summary => cloneFile(this.#files.get(summary.path)!))),
      fileCount: this.fileCount,
      byteLength: this.byteLength,
      sha256: this.sha256,
    })
  }
}
