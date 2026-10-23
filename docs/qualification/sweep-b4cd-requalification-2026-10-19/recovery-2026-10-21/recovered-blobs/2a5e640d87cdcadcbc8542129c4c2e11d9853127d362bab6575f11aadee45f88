export const MAX_PROJECT_FILES = 128
export const MAX_PROJECT_SOURCE_BYTES = 1_048_576
export const MAX_VIRTUAL_PATH_LENGTH = 256
export const MAX_VIRTUAL_SEGMENT_LENGTH = 96

export interface VirtualSourceFile {
  readonly path: string
  readonly source: string
}

function validateSegment(segment: string) {
  if (!segment || segment === '.') return false
  if (segment === '..') throw new Error('Virtual paths may not traverse above their project root')
  if (segment.length > MAX_VIRTUAL_SEGMENT_LENGTH || /[\u0000-\u001f\u007f]/.test(segment)) throw new Error('Virtual path segment is invalid')
  return true
}

function wellFormed(value: string) {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
}

/** Canonical project-relative POSIX path; host filesystem access is impossible. */
export function normalizeVirtualPath(path: string): string {
  if (typeof path !== 'string' || !path || path.length > MAX_VIRTUAL_PATH_LENGTH) throw new Error('Invalid virtual path')
  if (!wellFormed(path)) throw new Error('Virtual paths must contain well-formed Unicode')
  path = path.normalize('NFC')
  if (path.startsWith('/') || path.endsWith('/') || path.includes('//') || path.includes('\\')
    || /^[A-Za-z][A-Za-z0-9+.-]*:/.test(path)) throw new Error('Virtual paths must be project-relative POSIX paths')
  const segments: string[] = []
  for (const segment of path.split('/')) if (validateSegment(segment)) segments.push(segment)
  if (!segments.length) throw new Error('Invalid virtual path')
  const normalized = segments.join('/')
  if (normalized.length > MAX_VIRTUAL_PATH_LENGTH) throw new Error('Invalid virtual path')
  return normalized
}

export function resolveVirtualPath(importer: string, specifier: string): string {
  const from = normalizeVirtualPath(importer).split('/')
  from.pop()
  if (!wellFormed(specifier)) throw new Error('Virtual dependencies must contain well-formed Unicode')
  specifier = specifier.normalize('NFC')
  if (specifier.startsWith('/') || specifier.endsWith('/') || specifier.includes('//')
    || specifier.includes('\\') || /^[A-Za-z][A-Za-z0-9+.-]*:/.test(specifier)) {
    throw new Error('Virtual dependencies cannot access host or absolute paths')
  }
  const resolved = [...from]
  for (const segment of specifier.split('/')) {
    if (!segment || segment === '.') continue
    if (segment === '..') {
      if (!resolved.length) throw new Error('Virtual dependency escapes the project root')
      resolved.pop()
    } else {
      validateSegment(segment)
      resolved.push(segment)
    }
  }
  return normalizeVirtualPath(resolved.join('/'))
}

export class VirtualFileSystem {
  private readonly files = new Map<string, string>()

  constructor(files: readonly VirtualSourceFile[]) {
    if (files.length > MAX_PROJECT_FILES) throw new Error(`Project exceeds ${MAX_PROJECT_FILES} files`)
    let sourceBytes = 0
    for (const file of files) {
      const path = normalizeVirtualPath(file.path)
      if (this.files.has(path)) throw new Error(`Duplicate virtual file ${path}`)
      if (typeof file.source !== 'string') throw new Error(`Invalid source for ${path}`)
      if (!wellFormed(file.source)) throw new Error(`Source for ${path} must contain well-formed Unicode`)
      sourceBytes += new TextEncoder().encode(file.source).byteLength
      if (sourceBytes > MAX_PROJECT_SOURCE_BYTES) throw new Error(`Project exceeds ${MAX_PROJECT_SOURCE_BYTES} UTF-8 bytes`)
      this.files.set(path, file.source)
    }
  }

  read(path: string): string | null { return this.files.get(normalizeVirtualPath(path)) ?? null }

  resolve(importer: string, specifier: string): VirtualSourceFile | null {
    const path = resolveVirtualPath(importer, specifier)
    const source = this.files.get(path)
    return source === undefined ? null : { path, source }
  }

  list(): string[] { return [...this.files.keys()].sort() }
}
