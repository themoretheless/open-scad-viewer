import { FORMAT_SET, NUMBER, OPENSCAD_IMPORT_MAX_TRIANGLES, OPENSCAD_IMPORT_MAX_VERTICES, OpenScadImportDataError, UTF8, UTF8_FATAL } from '../openScadImport'
import type { OpenScadImportFormat, OpenScadImportGeometry, OpenScadImportGeometry3D } from '../openScadImport'
import type { OpenScadProjectFile } from '../openScadProject'

export function dataError(
  code: OpenScadImportDataError['code'],
  message: string,
  format?: OpenScadImportFormat,
  limit?: number,
  actual?: number,
): never {
  throw new OpenScadImportDataError(code, message, format, limit, actual)
}

export function finiteNumber(token: string, label: string, format: OpenScadImportFormat): number {
  if (!NUMBER.test(token)) return dataError('E_IMPORT_INVALID_DATA', `${label} is not a number.`, format)
  const value = Number(token)
  if (!Number.isFinite(value)) return dataError('E_IMPORT_INVALID_DATA', `${label} must be finite.`, format)
  return value
}

export function boundedCount(
  value: number,
  limit: number,
  label: string,
  format: OpenScadImportFormat,
): number {
  if (!Number.isSafeInteger(value) || value < 0) {
    return dataError('E_IMPORT_INVALID_DATA', `${label} must be a non-negative integer.`, format)
  }
  if (value > limit) {
    return dataError('E_IMPORT_LIMIT', `${label} exceeds the ${limit.toLocaleString('en-US')} limit.`, format, limit, value)
  }
  return value
}

export function bytesOf(file: OpenScadProjectFile): Uint8Array {
  return file.kind === 'blob' ? file.data : UTF8.encode(file.source)
}

export function textOf(file: OpenScadProjectFile, format: OpenScadImportFormat): string {
  if (file.kind === 'source') return file.source
  try {
    return UTF8_FATAL.decode(file.data)
  } catch {
    return dataError('E_IMPORT_ENCODING', `${format.toUpperCase()} input must be valid UTF-8 text.`, format)
  }
}

export function formatOfPath(path: string): OpenScadImportFormat | null {
  const dot = path.lastIndexOf('.')
  if (dot < 0) return null
  const format = path.slice(dot + 1).toLowerCase()
  return FORMAT_SET.has(format) ? format as OpenScadImportFormat : null
}

export function samePoint3(vertices: ArrayLike<number>, a: number, b: number): boolean {
  return vertices[a * 3] === vertices[b * 3]
    && vertices[a * 3 + 1] === vertices[b * 3 + 1]
    && vertices[a * 3 + 2] === vertices[b * 3 + 2]
}

export function triangleDegenerate(vertices: ArrayLike<number>, a: number, b: number, c: number): boolean {
  if (a === b || b === c || a === c || samePoint3(vertices, a, b)
    || samePoint3(vertices, b, c) || samePoint3(vertices, a, c)) return true
  const ax = vertices[a * 3], ay = vertices[a * 3 + 1], az = vertices[a * 3 + 2]
  const abx = vertices[b * 3] - ax, aby = vertices[b * 3 + 1] - ay, abz = vertices[b * 3 + 2] - az
  const acx = vertices[c * 3] - ax, acy = vertices[c * 3 + 1] - ay, acz = vertices[c * 3 + 2] - az
  const nx = aby * acz - abz * acy
  const ny = abz * acx - abx * acz
  const nz = abx * acy - aby * acx
  return nx === 0 && ny === 0 && nz === 0
}

export function geometry3D(
  format: OpenScadImportGeometry3D['format'],
  sourceVertices: readonly number[],
  sourceTriangles: readonly number[],
): OpenScadImportGeometry3D {
  if (sourceVertices.length % 3 !== 0 || sourceTriangles.length % 3 !== 0) {
    return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} mesh storage is inconsistent.`, format)
  }
  const vertexCount = boundedCount(
    sourceVertices.length / 3,
    OPENSCAD_IMPORT_MAX_VERTICES,
    `${format.toUpperCase()} vertex count`,
    format,
  )
  boundedCount(
    sourceTriangles.length / 3,
    OPENSCAD_IMPORT_MAX_TRIANGLES,
    `${format.toUpperCase()} triangle count`,
    format,
  )
  const vertices = new Float32Array(sourceVertices.length)
  for (let index = 0; index < sourceVertices.length; index++) {
    const value = sourceVertices[index]
    const stored = Math.fround(value)
    if (!Number.isFinite(value) || !Number.isFinite(stored)) {
      return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} contains a non-finite or out-of-range coordinate.`, format)
    }
    vertices[index] = stored
  }
  const triangles: number[] = []
  for (let offset = 0; offset < sourceTriangles.length; offset += 3) {
    const a = sourceTriangles[offset], b = sourceTriangles[offset + 1], c = sourceTriangles[offset + 2]
    if (![a, b, c].every(index => Number.isSafeInteger(index) && index >= 0 && index < vertexCount)) {
      return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} triangle index is out of bounds.`, format)
    }
    if (!triangleDegenerate(vertices, a, b, c)) triangles.push(a, b, c)
  }
  if (triangles.length === 0) {
    return dataError('E_IMPORT_EMPTY', `${format.toUpperCase()} contains no non-degenerate triangles.`, format)
  }
  return Object.freeze({
    dimension: 3 as const,
    format,
    vertices,
    triangles: Uint32Array.from(triangles),
    triangleCount: triangles.length / 3,
  })
}

export function cloneGeometry(geometry: OpenScadImportGeometry): OpenScadImportGeometry {
  if (geometry.dimension === 3) {
    return Object.freeze({
      ...geometry,
      vertices: new Float32Array(geometry.vertices),
      triangles: new Uint32Array(geometry.triangles),
    })
  }
  const regions = geometry.regions.map(region => Object.freeze({
    ...region,
    contours: Object.freeze(region.contours.map(contour => Object.freeze(
      contour.map(point => Object.freeze([point[0], point[1]] as const)),
    ))),
  }))
  return Object.freeze({ ...geometry, regions: Object.freeze(regions) })
}

export function startsWithSolid(bytes: Uint8Array): boolean {
  let offset = bytes.byteLength >= 3 && bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf ? 3 : 0
  while (offset < bytes.byteLength && /\s/u.test(String.fromCharCode(bytes[offset]))) offset++
  return String.fromCharCode(...bytes.subarray(offset, offset + 5)).toLowerCase() === 'solid'
}

/** Decode binary or ASCII STL into bounded indexed-neutral triangle geometry. */
export function parseOpenScadStl(file: OpenScadProjectFile): OpenScadImportGeometry3D {
  const bytes = bytesOf(file)
  const binarySizeMatches = bytes.byteLength >= 84
    && 84 + new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(80, true) * 50 === bytes.byteLength
  if (binarySizeMatches) return parseBinaryOpenScadStl(bytes)
  if (startsWithSolid(bytes)) return parseAsciiOpenScadStl(textOf(file, 'stl'))
  return dataError('E_IMPORT_INVALID_DATA', 'STL is neither a size-consistent binary file nor an ASCII solid.', 'stl')
}

export function parseBinaryOpenScadStl(bytes: Uint8Array): OpenScadImportGeometry3D {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const count = boundedCount(view.getUint32(80, true), OPENSCAD_IMPORT_MAX_TRIANGLES, 'STL triangle count', 'stl')
  if (84 + count * 50 !== bytes.byteLength) {
    return dataError('E_IMPORT_INVALID_DATA', 'Binary STL size does not match its declared triangle count.', 'stl')
  }
  const vertices: number[] = []
  const triangles: number[] = []
  let record = 84
  for (let triangle = 0; triangle < count; triangle++, record += 50) {
    const base = vertices.length / 3
    for (let channel = 0; channel < 9; channel++) {
      const value = view.getFloat32(record + 12 + channel * 4, true)
      if (!Number.isFinite(value)) {
        return dataError('E_IMPORT_INVALID_DATA', `STL triangle ${triangle + 1} contains a non-finite coordinate.`, 'stl')
      }
      vertices.push(value)
    }
    triangles.push(base, base + 1, base + 2)
  }
  return geometry3D('stl', vertices, triangles)
}

export function parseAsciiOpenScadStl(text: string): OpenScadImportGeometry3D {
  const vertices: number[] = []
  const triangles: number[] = []
  let facetVertices: number[] = []
  let state: 'before' | 'solid' | 'facet' | 'loop' | 'after-loop' | 'ended' = 'before'
  for (const raw of text.replace(/^\uFEFF/u, '').split(/\r\n|\n|\r/u)) {
    const line = raw.trim()
    if (!line) continue
    const parts = line.split(/\s+/u)
    const keyword = parts[0].toLowerCase()
    if (keyword === 'solid') {
      if (state !== 'before') return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL contains a misplaced solid declaration.', 'stl')
      state = 'solid'
    } else if (keyword === 'endsolid') {
      if (state !== 'solid') return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL ended inside a facet or outside a solid.', 'stl')
      state = 'ended'
    } else if (keyword === 'facet') {
      if (state !== 'solid' || parts[1]?.toLowerCase() !== 'normal' || parts.length !== 5) {
        return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL contains an invalid facet declaration.', 'stl')
      }
      for (const token of parts.slice(2)) finiteNumber(token, 'STL facet normal', 'stl')
      state = 'facet'
    } else if (keyword === 'outer') {
      if (state !== 'facet' || parts.length !== 2 || parts[1].toLowerCase() !== 'loop') {
        return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL contains an invalid outer loop.', 'stl')
      }
      state = 'loop'
    } else if (keyword === 'vertex') {
      if (state !== 'loop' || parts.length !== 4 || facetVertices.length >= 9) {
        return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL facet must contain exactly three vertices.', 'stl')
      }
      facetVertices.push(
        finiteNumber(parts[1], 'STL vertex', 'stl'),
        finiteNumber(parts[2], 'STL vertex', 'stl'),
        finiteNumber(parts[3], 'STL vertex', 'stl'),
      )
    } else if (keyword === 'endloop') {
      if (state !== 'loop' || parts.length !== 1 || facetVertices.length !== 9) {
        return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL loop must contain exactly three vertices.', 'stl')
      }
      state = 'after-loop'
    } else if (keyword === 'endfacet') {
      if (state !== 'after-loop' || parts.length !== 1 || facetVertices.length !== 9) {
        return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL facet must contain exactly three vertices.', 'stl')
      }
      boundedCount(triangles.length / 3 + 1, OPENSCAD_IMPORT_MAX_TRIANGLES, 'STL triangle count', 'stl')
      const base = vertices.length / 3
      vertices.push(...facetVertices)
      triangles.push(base, base + 1, base + 2)
      facetVertices = []
      state = 'solid'
    } else {
      return dataError('E_IMPORT_INVALID_DATA', `ASCII STL contains unsupported statement ${JSON.stringify(parts[0])}.`, 'stl')
    }
  }
  if (state !== 'ended' || facetVertices.length !== 0) {
    return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL is incomplete.', 'stl')
  }
  return geometry3D('stl', vertices, triangles)
}

export function offLines(text: string): string[][] {
  const lines: string[][] = []
  for (const raw of text.replace(/^\uFEFF/u, '').split(/\r\n|\n|\r/u)) {
    const line = raw.replace(/#.*/u, '').trim()
    if (line) lines.push(line.split(/\s+/u))
  }
  return lines
}

export function polygonNormal(vertices: readonly number[], face: readonly number[]): [number, number, number] {
  let nx = 0, ny = 0, nz = 0
  for (let index = 0; index < face.length; index++) {
    const current = face[index] * 3
    const next = face[(index + 1) % face.length] * 3
    const x = vertices[current], y = vertices[current + 1], z = vertices[current + 2]
    const xx = vertices[next], yy = vertices[next + 1], zz = vertices[next + 2]
    nx += (y - yy) * (z + zz)
    ny += (z - zz) * (x + xx)
    nz += (x - xx) * (y + yy)
  }
  return [nx, ny, nz]
}

export function triangulateFace3D(vertices: readonly number[], face: readonly number[], format: OpenScadImportFormat): number[] {
  if (face.length === 3) return [...face]
  const normal = polygonNormal(vertices, face)
  const axis = Math.abs(normal[0]) >= Math.abs(normal[1]) && Math.abs(normal[0]) >= Math.abs(normal[2])
    ? 0
    : Math.abs(normal[1]) >= Math.abs(normal[2]) ? 1 : 2
  const project = (index: number): [number, number] => {
    const offset = index * 3
    return axis === 0
      ? [vertices[offset + 1], vertices[offset + 2]]
      : axis === 1 ? [vertices[offset], vertices[offset + 2]] : [vertices[offset], vertices[offset + 1]]
  }
  const points = face.map(project)
  let area = 0
  for (let index = 0; index < points.length; index++) {
    const a = points[index], b = points[(index + 1) % points.length]
    area += a[0] * b[1] - b[0] * a[1]
  }
  if (area === 0) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} contains a degenerate polygon face.`, format)
  const orientation = Math.sign(area)
  const remaining = [...face.keys()]
  const output: number[] = []
  const cross = (a: readonly number[], b: readonly number[], c: readonly number[]) => (
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
  )
  const inside = (point: readonly number[], a: readonly number[], b: readonly number[], c: readonly number[]) => {
    const ab = cross(a, b, point) * orientation
    const bc = cross(b, c, point) * orientation
    const ca = cross(c, a, point) * orientation
    return ab >= 0 && bc >= 0 && ca >= 0
  }
  while (remaining.length > 3) {
    let clipped = false
    for (let cursor = 0; cursor < remaining.length; cursor++) {
      const before = remaining[(cursor + remaining.length - 1) % remaining.length]
      const current = remaining[cursor]
      const after = remaining[(cursor + 1) % remaining.length]
      const a = points[before], b = points[current], c = points[after]
      if (cross(a, b, c) * orientation <= 0) continue
      if (remaining.some(candidate => candidate !== before && candidate !== current && candidate !== after
        && inside(points[candidate], a, b, c))) continue
      output.push(face[before], face[current], face[after])
      remaining.splice(cursor, 1)
      clipped = true
      break
    }
    if (!clipped) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} contains a self-intersecting polygon face.`, format)
  }
  output.push(face[remaining[0]], face[remaining[1]], face[remaining[2]])
  return output
}

/** Parse ASCII OFF/COFF geometry, including bounded concave polygon faces. */
export function parseOpenScadOff(file: OpenScadProjectFile): OpenScadImportGeometry3D {
  const lines = offLines(textOf(file, 'off'))
  let lineIndex = 0
  const headerLine = lines[lineIndex++] ?? []
  const header = headerLine.shift()?.toUpperCase()
  if (header !== 'OFF' && header !== 'COFF') {
    return dataError('E_IMPORT_INVALID_DATA', 'OFF input must begin with OFF or COFF.', 'off')
  }
  const counts = headerLine.length ? headerLine : lines[lineIndex++] ?? []
  if (counts.length < 3) return dataError('E_IMPORT_INVALID_DATA', 'OFF input is missing its counts line.', 'off')
  const vertexCount = boundedCount(
    finiteNumber(counts[0], 'OFF vertex count', 'off'),
    OPENSCAD_IMPORT_MAX_VERTICES,
    'OFF vertex count',
    'off',
  )
  const faceCount = boundedCount(
    finiteNumber(counts[1], 'OFF face count', 'off'),
    OPENSCAD_IMPORT_MAX_TRIANGLES,
    'OFF face count',
    'off',
  )
  boundedCount(finiteNumber(counts[2], 'OFF edge count', 'off'), Number.MAX_SAFE_INTEGER, 'OFF edge count', 'off')
  const vertices: number[] = []
  for (let vertex = 0; vertex < vertexCount; vertex++) {
    const fields = lines[lineIndex++] ?? []
    if (fields.length < 3) return dataError('E_IMPORT_INVALID_DATA', `OFF vertex ${vertex + 1} is incomplete.`, 'off')
    vertices.push(
      finiteNumber(fields[0], `OFF vertex ${vertex + 1} x`, 'off'),
      finiteNumber(fields[1], `OFF vertex ${vertex + 1} y`, 'off'),
      finiteNumber(fields[2], `OFF vertex ${vertex + 1} z`, 'off'),
    )
    // OFF variants append normals/colors/texture coordinates per vertex. They do not affect imported geometry.
    for (const token of fields.slice(3)) finiteNumber(token, `${header} vertex attribute`, 'off')
  }
  const triangles: number[] = []
  for (let faceIndex = 0; faceIndex < faceCount; faceIndex++) {
    const fields = lines[lineIndex++] ?? []
    const size = boundedCount(
      finiteNumber(fields[0] ?? '', `OFF face ${faceIndex + 1} size`, 'off'),
      OPENSCAD_IMPORT_MAX_VERTICES,
      `OFF face ${faceIndex + 1} size`,
      'off',
    )
    if (size < 3) return dataError('E_IMPORT_INVALID_DATA', 'OFF faces require at least three vertices.', 'off')
    if (fields.length < size + 1) return dataError('E_IMPORT_INVALID_DATA', `OFF face ${faceIndex + 1} is incomplete.`, 'off')
    const face: number[] = []
    for (let index = 0; index < size; index++) {
      const value = finiteNumber(fields[index + 1], 'OFF face index', 'off')
      if (!Number.isSafeInteger(value) || value < 0 || value >= vertexCount) {
        return dataError('E_IMPORT_INVALID_DATA', 'OFF face index is out of bounds.', 'off')
      }
      face.push(value)
    }
    for (const token of fields.slice(size + 1)) finiteNumber(token, 'OFF face attribute', 'off')
    triangles.push(...triangulateFace3D(vertices, face, 'off'))
    boundedCount(triangles.length / 3, OPENSCAD_IMPORT_MAX_TRIANGLES, 'OFF triangle count', 'off')
  }
  // OFF permits optional colors after faces. They are intentionally ignored.
  return geometry3D('off', vertices, triangles)
}

export function xmlNameCharacter(value: string, first: boolean): boolean {
  return first ? /[A-Za-z_:]/u.test(value) : /[A-Za-z0-9_.:-]/u.test(value)
}

export function decodeXmlEntities(value: string, format: OpenScadImportFormat): string {
  if (/&(?!(?:amp|lt|gt|quot|apos|#[0-9]+|#x[0-9a-f]+);)/iu.test(value)) {
    return dataError('E_IMPORT_INVALID_DATA', 'XML contains a malformed or unsupported entity reference.', format)
  }
  return value.replace(/&([^;]{1,32});/gu, (_whole, entity: string) => {
    if (entity === 'amp') return '&'
    if (entity === 'lt') return '<'
    if (entity === 'gt') return '>'
    if (entity === 'quot') return '"'
    if (entity === 'apos') return "'"
    let code = -1
    if (/^#[0-9]+$/u.test(entity)) code = Number(entity.slice(1))
    else if (/^#x[0-9a-f]+$/iu.test(entity)) code = Number.parseInt(entity.slice(2), 16)
    if (Number.isSafeInteger(code) && code >= 0 && code <= 0x10ffff
      && !(code >= 0xd800 && code <= 0xdfff)) return String.fromCodePoint(code)
    return dataError('E_IMPORT_INVALID_DATA', `XML contains unsupported entity &${entity};.`, format)
  })
}
