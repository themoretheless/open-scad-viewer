import { OPENSCAD_2021_GRID_FINE, OPENSCAD_TEXT_MAX_ABS_SIZE, OPENSCAD_TEXT_MAX_CODE_POINTS, OPENSCAD_TEXT_MAX_CURVE_SEGMENTS, OPENSCAD_TEXT_MAX_FACES_PER_COLLECTION, OPENSCAD_TEXT_MAX_FONTS, OPENSCAD_TEXT_MAX_NAME_RECORDS, OPENSCAD_TEXT_MAX_NAME_TABLE_BYTES, OpenScadTextError, loadHarfBuzz, preparedRuntimes } from '../openScadText'
import type { HarfBuzzApi, HarfBuzzFace, OpenScadTextErrorCode, OpenScadTextErrorDetails, OpenScadTextFont, ParsedFontQuery, PreparedOpenScadTextAssets, RejectedOpenScadTextFont, SfntFace } from '../openScadText'
import { OpenScadProject, resolveOpenScadProjectPath } from '../openScadProject'
import type { OpenScadProjectBlobFile } from '../openScadProject'
import { u32 } from '../../core/bytes'

export function textError(
  code: OpenScadTextErrorCode,
  message: string,
  details: OpenScadTextErrorDetails = {},
  cause?: unknown,
): never {
  throw new OpenScadTextError(code, message, details, cause === undefined ? {} : { cause })
}

export function u16(bytes: Uint8Array, offset: number): number {
  return bytes[offset] * 0x100 + bytes[offset + 1]
}

export function asciiTag(bytes: Uint8Array, offset: number): string {
  return String.fromCharCode(bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3])
}

export function invalidFont(
  path: string,
  message: string,
  code: 'E_TEXT_FONT_INVALID' | 'E_TEXT_FONT_UNSUPPORTED' = 'E_TEXT_FONT_INVALID',
): never {
  return textError(code, `Font ${path} ${message}`, { fontPath: path })
}

export function validateSfntFace(bytes: Uint8Array, path: string, offset: number): SfntFace {
  if (offset < 0 || offset + 12 > bytes.byteLength) invalidFont(path, 'has a truncated SFNT header.')
  const signature = asciiTag(bytes, offset)
  if (u32(bytes, offset) !== 0x00010000 && signature !== 'OTTO' && signature !== 'true') {
    invalidFont(path, 'uses an unsupported SFNT face signature.', 'E_TEXT_FONT_UNSUPPORTED')
  }
  const tableCount = u16(bytes, offset + 4)
  if (tableCount === 0 || tableCount > 256) invalidFont(path, 'has an invalid SFNT table count.')
  const directoryEnd = offset + 12 + tableCount * 16
  if (!Number.isSafeInteger(directoryEnd) || directoryEnd > bytes.byteLength) {
    invalidFont(path, 'has a truncated SFNT table directory.')
  }

  const tables = new Map<string, { offset: number; length: number }>()
  const occupied: Array<{ start: number; end: number; tag: string }> = []
  for (let index = 0; index < tableCount; index++) {
    const entry = offset + 12 + index * 16
    const tag = asciiTag(bytes, entry)
    const tableOffset = u32(bytes, entry + 8)
    const length = u32(bytes, entry + 12)
    const end = tableOffset + length
    if (!Number.isSafeInteger(end) || tableOffset > bytes.byteLength || end > bytes.byteLength) {
      invalidFont(path, `has an out-of-bounds ${JSON.stringify(tag)} table.`)
    }
    if (tables.has(tag)) invalidFont(path, `has duplicate ${JSON.stringify(tag)} tables.`)
    tables.set(tag, { offset: tableOffset, length })
    if (length > 0) occupied.push({ start: tableOffset, end, tag })
  }
  occupied.sort((left, right) => left.start - right.start || left.end - right.end)
  for (let index = 1; index < occupied.length; index++) {
    const previous = occupied[index - 1]
    const current = occupied[index]
    if (current.start < previous.end) {
      invalidFont(path, `has overlapping ${JSON.stringify(previous.tag)} and ${JSON.stringify(current.tag)} tables.`)
    }
  }

  const head = tables.get('head')
  const maxp = tables.get('maxp')
  const cmap = tables.get('cmap')
  const name = tables.get('name')
  if (!head || head.length < 54 || !maxp || maxp.length < 6 || !cmap || cmap.length < 4) {
    invalidFont(path, 'is missing a required head, maxp, or cmap table.')
  }
  const hasTrueTypeOutlines = tables.has('glyf') && tables.has('loca')
  const hasCffOutlines = tables.has('CFF ') || tables.has('CFF2')
  if (!hasTrueTypeOutlines && !hasCffOutlines) {
    invalidFont(path, 'has no supported TrueType or CFF outline tables.', 'E_TEXT_FONT_UNSUPPORTED')
  }
  if (name && name.length > OPENSCAD_TEXT_MAX_NAME_TABLE_BYTES) {
    textError(
      'E_TEXT_FONT_LIMIT',
      `Font ${path} name table exceeds ${OPENSCAD_TEXT_MAX_NAME_TABLE_BYTES.toLocaleString()} bytes.`,
      { fontPath: path, limit: OPENSCAD_TEXT_MAX_NAME_TABLE_BYTES, actual: name.length },
    )
  }
  const unitsPerEm = u16(bytes, head.offset + 18)
  if (unitsPerEm < 16 || unitsPerEm > 16_384) invalidFont(path, 'has an invalid units-per-em value.')
  const glyphCount = u16(bytes, maxp.offset + 4)
  if (glyphCount === 0) invalidFont(path, 'contains no glyphs.')
  if (hasTrueTypeOutlines) {
    const loca = tables.get('loca')!
    const indexToLocFormat = u16(bytes, head.offset + 50)
    if (indexToLocFormat !== 0 && indexToLocFormat !== 1) invalidFont(path, 'has an invalid loca format.')
    const requiredLocaBytes = (glyphCount + 1) * (indexToLocFormat === 0 ? 2 : 4)
    if (loca.length < requiredLocaBytes) invalidFont(path, 'has a truncated loca table.')
  }
  return Object.freeze({ offset, unitsPerEm })
}

export function validateFont(bytes: Uint8Array, path: string): readonly SfntFace[] {
  if (bytes.byteLength < 12) invalidFont(path, 'is truncated.')
  if (asciiTag(bytes, 0) !== 'ttcf') return Object.freeze([validateSfntFace(bytes, path, 0)])

  const version = u32(bytes, 4)
  if (version !== 0x00010000 && version !== 0x00020000) {
    invalidFont(path, 'uses an unsupported TrueType collection version.', 'E_TEXT_FONT_UNSUPPORTED')
  }
  const count = u32(bytes, 8)
  if (count === 0 || count > OPENSCAD_TEXT_MAX_FACES_PER_COLLECTION) {
    textError(
      'E_TEXT_FONT_LIMIT',
      `Font collection ${path} exceeds ${OPENSCAD_TEXT_MAX_FACES_PER_COLLECTION} faces.`,
      { fontPath: path, limit: OPENSCAD_TEXT_MAX_FACES_PER_COLLECTION, actual: count },
    )
  }
  const directoryEnd = 12 + count * 4
  if (!Number.isSafeInteger(directoryEnd) || directoryEnd > bytes.byteLength) {
    invalidFont(path, 'has a truncated TrueType collection directory.')
  }
  const offsets = new Set<number>()
  const faces: SfntFace[] = []
  for (let index = 0; index < count; index++) {
    const offset = u32(bytes, 12 + index * 4)
    if (offsets.has(offset)) invalidFont(path, 'contains duplicate face offsets.')
    offsets.add(offset)
    faces.push(validateSfntFace(bytes, path, offset))
  }
  return Object.freeze(faces)
}

export function fontExtension(path: string): boolean {
  return /\.(?:ttf|otf|ttc|otc)$/iu.test(path)
}

export function canonicalName(value: string): string {
  return value.normalize('NFKC').trim().replace(/\s+/gu, ' ').toLocaleLowerCase('en-US')
}

export function safeFontName(
  face: HarfBuzzFace,
  entries: readonly { nameId: number; language: string }[],
  nameId: number,
): string | undefined {
  const matches = entries.filter(entry => entry.nameId === nameId)
  const selected = matches.find(entry => /^en(?:-|$)/iu.test(entry.language)) ?? matches[0]
  if (!selected) return undefined
  const value = face.getName(nameId, selected.language).replace(/\u0000/gu, '').trim()
  if (value.length > 1_024) throw new Error(`name ID ${nameId} exceeds 1024 characters`)
  return value || undefined
}

export function describeFace(
  hb: HarfBuzzApi,
  file: OpenScadProjectBlobFile,
  faceIndex: number,
  validated: SfntFace,
): OpenScadTextFont {
  const blob = hb.createBlob(file.data)
  let face: HarfBuzzFace | undefined
  try {
    face = hb.createFace(blob, faceIndex)
    if (face.upem !== validated.unitsPerEm) throw new Error('HarfBuzz reported inconsistent units-per-em')
    const entries = face.listNames()
    if (entries.length > OPENSCAD_TEXT_MAX_NAME_RECORDS) {
      textError(
        'E_TEXT_FONT_LIMIT',
        `Font ${file.path} exceeds ${OPENSCAD_TEXT_MAX_NAME_RECORDS} name records.`,
        {
          fontPath: file.path,
          faceIndex,
          limit: OPENSCAD_TEXT_MAX_NAME_RECORDS,
          actual: entries.length,
        },
      )
    }
    const filename = file.path.slice(file.path.lastIndexOf('/') + 1).replace(/\.[^.]+$/u, '')
    const family = safeFontName(face, entries, 16) ?? safeFontName(face, entries, 1) ?? filename
    const style = safeFontName(face, entries, 17) ?? safeFontName(face, entries, 2) ?? 'Regular'
    return Object.freeze({
      path: file.path,
      sha256: file.sha256,
      faceIndex,
      family,
      style,
      fullName: safeFontName(face, entries, 4) ?? `${family} ${style}`.trim(),
      postscriptName: safeFontName(face, entries, 6) ?? `${family}-${style}`.replace(/\s+/gu, ''),
      unitsPerEm: face.upem,
    })
  } finally {
    face?.destroy()
    blob.destroy()
  }
}

/** Validate and index every bounded VFS font before the synchronous evaluator. */
export async function prepareOpenScadTextAssets(
  project: OpenScadProject,
): Promise<PreparedOpenScadTextAssets> {
  if (!(project instanceof OpenScadProject)) {
    return textError('E_TEXT_PROJECT_REQUIRED', 'text() requires an immutable OpenScadProject VFS.')
  }
  const fontPaths = project.list()
    .filter(file => file.kind === 'blob' && fontExtension(file.path))
    .map(file => file.path)
  if (fontPaths.length > OPENSCAD_TEXT_MAX_FONTS) {
    return textError(
      'E_TEXT_FONT_LIMIT',
      `OpenSCAD project exceeds ${OPENSCAD_TEXT_MAX_FONTS} font files.`,
      { limit: OPENSCAD_TEXT_MAX_FONTS, actual: fontPaths.length },
    )
  }

  const fonts: OpenScadTextFont[] = []
  const rejected: RejectedOpenScadTextFont[] = []
  const hb = fontPaths.length === 0 ? undefined : await loadHarfBuzz()
  for (const path of fontPaths) {
    const file = project.read(path)
    if (file?.kind !== 'blob') continue
    try {
      const faces = validateFont(file.data, path)
      for (let faceIndex = 0; faceIndex < faces.length; faceIndex++) {
        fonts.push(describeFace(hb!, file, faceIndex, faces[faceIndex]))
      }
    } catch (error) {
      const typed = error instanceof OpenScadTextError
        ? error
        : new OpenScadTextError(
          'E_TEXT_FONT_INVALID',
          `Font ${path} could not be read safely: ${error instanceof Error ? error.message : String(error)}`,
          { fontPath: path },
          { cause: error },
        )
      rejected.push(Object.freeze({
        path,
        code: typed.code === 'E_TEXT_FONT_UNSUPPORTED' ? typed.code : 'E_TEXT_FONT_INVALID',
        message: typed.message,
      }))
    }
  }
  fonts.sort((left, right) => left.path.localeCompare(right.path) || left.faceIndex - right.faceIndex)
  rejected.sort((left, right) => left.path.localeCompare(right.path))
  const prepared = Object.freeze({
    projectSha256: project.sha256,
    fonts: Object.freeze(fonts),
    rejected: Object.freeze(rejected),
  })
  if (hb) preparedRuntimes.set(prepared, hb)
  return prepared
}

export function parseFontQuery(value: string): ParsedFontQuery {
  const fields = value.split(':')
  const family = fields.shift()?.trim() ?? ''
  let style: string | undefined
  for (const field of fields) {
    const match = /^\s*style\s*=\s*(.*?)\s*$/iu.exec(field)
    if (match) style = match[1]
  }
  return Object.freeze({ original: value, family, ...(style ? { style } : {}) })
}

export function exactFontPaths(project: OpenScadProject, sourcePath: string, query: string): readonly string[] {
  if (!fontExtension(query) || query.includes(':')) return []
  const paths: string[] = []
  try {
    paths.push(resolveOpenScadProjectPath(sourcePath, query))
  } catch {
    // The family-name resolver below will produce the stable public error.
  }
  if (!query.startsWith('.')) {
    try {
      if (project.read(query)?.kind === 'blob') paths.push(query)
    } catch {
      // Reject absolute/traversing candidates through the typed font resolver.
    }
  }
  return Object.freeze([...new Set(paths)])
}

export function resolveFont(
  project: OpenScadProject,
  prepared: PreparedOpenScadTextAssets,
  sourcePath: string,
  value: string,
): OpenScadTextFont {
  const exactPaths = exactFontPaths(project, sourcePath, value.trim())
  for (const path of exactPaths) {
    const match = prepared.fonts.find(font => font.path === path)
    if (match) return match
    const rejected = prepared.rejected.find(font => font.path === path)
    if (rejected) return textError(rejected.code, rejected.message, { sourcePath, font: value, fontPath: path })
  }

  const query = parseFontQuery(value)
  if (query.family.length === 0) {
    const regular = prepared.fonts.find(font => /^(?:regular|normal|book)$/iu.test(font.style))
    const selected = regular ?? prepared.fonts[0]
    if (selected) return selected
  } else {
    const family = canonicalName(query.family)
    const style = query.style === undefined ? undefined : canonicalName(query.style)
    const scored = prepared.fonts.flatMap(font => {
      const filename = font.path.slice(font.path.lastIndexOf('/') + 1).replace(/\.[^.]+$/u, '')
      const names = [font.family, font.fullName, font.postscriptName, filename].map(canonicalName)
      const familyScore = names[0] === family ? 4
        : names[1] === family || names[2] === family ? 3
          : names[3] === family ? 2
            : names.some(name => name.startsWith(`${family} `) || name.startsWith(`${family}-`)) ? 1 : 0
      if (familyScore === 0) return []
      const styleMatches = style === undefined || canonicalName(font.style) === style
        || canonicalName(font.fullName).endsWith(` ${style}`)
      if (!styleMatches) return []
      const regularBonus = style === undefined && /^(?:regular|normal|book)$/iu.test(font.style) ? 1 : 0
      return [{ font, score: familyScore * 2 + regularBonus }]
    }).sort((left, right) => right.score - left.score
      || left.font.path.localeCompare(right.font.path)
      || left.font.faceIndex - right.font.faceIndex)
    if (scored[0]) return scored[0].font
  }

  if (prepared.fonts.length === 0 && prepared.rejected[0]) {
    const rejected = prepared.rejected[0]
    return textError(rejected.code, rejected.message, {
      sourcePath,
      font: value,
      fontPath: rejected.path,
    })
  }
  return textError(
    'E_TEXT_FONT_NOT_FOUND',
    value.trim().length === 0
      ? 'text() could not resolve a default font from the bounded project VFS.'
      : `text() could not resolve font ${JSON.stringify(value)} from the bounded project VFS.`,
    {
      sourcePath,
      font: value,
      ...(exactPaths[0] === undefined ? {} : { fontPath: exactPaths[0] }),
    },
  )
}

export function finiteParameter(value: number, name: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return textError('E_TEXT_PARAMETER_INVALID', `text() ${name} must be a finite number.`)
  }
  return value
}

export function boundedParameter(value: number, name: string, limit: number): number {
  finiteParameter(value, name)
  if (Math.abs(value) > limit) {
    return textError(
      'E_TEXT_PARAMETER_LIMIT',
      `text() ${name} exceeds the absolute limit ${limit.toLocaleString()}.`,
      { limit, actual: Math.abs(value) },
    )
  }
  return value
}

/** OpenSCAD 2021.01 text curve subdivision derived from $fn/$fa/$fs. */
export function calculateOpenScadTextSegments(
  size: number,
  fn = 0,
  fs = 2,
  fa = 12,
): number {
  boundedParameter(size, 'size', OPENSCAD_TEXT_MAX_ABS_SIZE)
  finiteParameter(fn, '$fn')
  finiteParameter(fs, '$fs')
  finiteParameter(fa, '$fa')
  let fragments: number
  if (size < OPENSCAD_2021_GRID_FINE) fragments = 3
  else if (fn > 0) fragments = Math.trunc(Math.max(fn, 3))
  else {
    if (fs <= 0 || fa <= 0) {
      return textError('E_TEXT_PARAMETER_INVALID', 'text() $fa and $fs must be positive when $fn is zero.')
    }
    fragments = Math.ceil(Math.max(Math.min(360 / fa, size * 2 * Math.PI / fs), 5))
  }
  const segments = Math.max(Math.floor(fragments / 8) + 1, 2)
  if (segments > OPENSCAD_TEXT_MAX_CURVE_SEGMENTS) {
    return textError(
      'E_TEXT_PARAMETER_LIMIT',
      `text() curve subdivision exceeds ${OPENSCAD_TEXT_MAX_CURVE_SEGMENTS} segments.`,
      { limit: OPENSCAD_TEXT_MAX_CURVE_SEGMENTS, actual: segments },
    )
  }
  return segments
}

export function codePointCount(text: string): number {
  let count = 0
  for (const _character of text) {
    count++
    if (count > OPENSCAD_TEXT_MAX_CODE_POINTS) return count
  }
  return count
}
