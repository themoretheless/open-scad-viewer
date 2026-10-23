/** Measured intrinsics in the original, EXIF-oriented raster. No camera inference. */
export interface PhotoCalibrationGroup {
  id: string
  label: string
  source: string
  measuredAt?: string
  imageWidth: number
  imageHeight: number
  fx: number
  fy: number
  cx: number
  cy: number
  distortion: {model: 'brown-conrady'; k1: number; k2: number; k3: number; p1: number; p2: number}
}

export interface PhotoCalibrationDocument {
  format: 'open-scad-viewer/photo-calibration'
  version: 1
  coordinateSystem: 'oriented-pixel-centers'
  groups: PhotoCalibrationGroup[]
}

export interface PhotoMeasuredInput {
  group: PhotoCalibrationGroup
  sourceWidth: number
  sourceHeight: number
}

export const MAX_CALIBRATION_FILE_BYTES = 128 * 1024
const FORMAT = 'open-scad-viewer/photo-calibration'

function record(value: unknown, name: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error(`${name}: expected an object`)
  return value as Record<string, unknown>
}
function keys(value: Record<string, unknown>, allowed: string[], name: string): void {
  const unknown = Object.keys(value).find(key => !allowed.includes(key))
  if (unknown) throw new Error(`${name}: unsupported field ${unknown}`)
}
function text(value: unknown, name: string, max: number): string {
  if (typeof value !== 'string' || !value.trim() || value.length > max) throw new Error(`${name}: expected non-empty text (at most ${max} characters)`)
  return value
}
function number(value: unknown, name: string, min: number, max: number): number {
  if (typeof value !== 'number' || !Number.isFinite(value) || value < min || value > max) throw new Error(`${name}: expected a finite value between ${min} and ${max}`)
  return value
}

/** Strict schema avoids mistaking fisheye/rational coefficients for Brown data. */
export function parsePhotoCalibration(json: string): PhotoCalibrationDocument {
  if (new TextEncoder().encode(json).length > MAX_CALIBRATION_FILE_BYTES) throw new Error('Calibration JSON exceeds 128 KiB')
  const document = record(JSON.parse(json), 'Calibration')
  keys(document, ['format', 'version', 'coordinateSystem', 'groups'], 'Calibration')
  if (document.format !== FORMAT || document.version !== 1 || document.coordinateSystem !== 'oriented-pixel-centers') {
    throw new Error('Use photo-calibration version 1 with oriented-pixel-centers coordinates')
  }
  if (!Array.isArray(document.groups) || document.groups.length < 1 || document.groups.length > 24) throw new Error('Calibration requires 1–24 measured groups')
  const ids = new Set<string>()
  const groups = document.groups.map((value, index): PhotoCalibrationGroup => {
    const name = `Calibration group ${index + 1}`
    const group = record(value, name)
    keys(group, ['id', 'label', 'source', 'measuredAt', 'imageWidth', 'imageHeight', 'fx', 'fy', 'cx', 'cy', 'distortion'], name)
    const id = text(group.id, `${name}.id`, 64)
    if (!/^[a-zA-Z0-9][a-zA-Z0-9_.:-]*$/.test(id) || ids.has(id)) throw new Error(`${name}: id must be unique and use letters, digits, dot, colon, underscore or hyphen`)
    ids.add(id)
    const imageWidth = number(group.imageWidth, `${name}.imageWidth`, 48, 50_000_000)
    const imageHeight = number(group.imageHeight, `${name}.imageHeight`, 48, 50_000_000)
    if (!Number.isInteger(imageWidth) || !Number.isInteger(imageHeight) || imageWidth * imageHeight > 50_000_000) throw new Error(`${name}: original image must be at most 50 megapixels`)
    const distortion = record(group.distortion, `${name}.distortion`)
    keys(distortion, ['model', 'k1', 'k2', 'k3', 'p1', 'p2'], `${name}.distortion`)
    if (distortion.model !== 'brown-conrady') throw new Error(`${name}: only Brown-Conrady distortion is supported`)
    const parsed: PhotoCalibrationGroup = {
      id, label: text(group.label, `${name}.label`, 120), source: text(group.source, `${name}.source`, 2000),
      imageWidth, imageHeight,
      fx: number(group.fx, `${name}.fx`, 1, 1e7), fy: number(group.fy, `${name}.fy`, 1, 1e7),
      cx: number(group.cx, `${name}.cx`, 0, imageWidth - 1), cy: number(group.cy, `${name}.cy`, 0, imageHeight - 1),
      distortion: {model: 'brown-conrady',
        k1: number(distortion.k1, `${name}.k1`, -10, 10), k2: number(distortion.k2, `${name}.k2`, -10, 10),
        k3: number(distortion.k3, `${name}.k3`, -10, 10), p1: number(distortion.p1, `${name}.p1`, -10, 10),
        p2: number(distortion.p2, `${name}.p2`, -10, 10)},
    }
    if (group.measuredAt !== undefined) parsed.measuredAt = text(group.measuredAt, `${name}.measuredAt`, 100)
    return parsed
  })
  return {format: FORMAT, version: 1, coordinateSystem: 'oriented-pixel-centers', groups}
}

export function measuredPhotoInput(group: PhotoCalibrationGroup, sourceWidth: number, sourceHeight: number): PhotoMeasuredInput {
  if (group.imageWidth !== sourceWidth || group.imageHeight !== sourceHeight) {
    throw new Error(`${group.label}: calibration is for ${group.imageWidth}×${group.imageHeight}; oriented photo is ${sourceWidth}×${sourceHeight}`)
  }
  return {group, sourceWidth, sourceHeight}
}

/** Deliberately synthetic numbers: this is a file-format example, not a lens profile. */
export function photoCalibrationExampleJson(): string {
  const first: PhotoCalibrationGroup = {
    id: 'synthetic-a', label: 'DEMO: synthetic camera A',
    source: 'Synthetic test values only. Replace with measured calibration; not Canon/iPhone measurements.',
    imageWidth: 320, imageHeight: 240, fx: 245, fy: 270, cx: 157, cy: 122,
    distortion: {model: 'brown-conrady', k1: 0.42, k2: 0.04, k3: 0.002, p1: 0.015, p2: -0.01},
  }
  const second: PhotoCalibrationGroup = {...first, id: 'synthetic-b', label: 'DEMO: synthetic camera B',
    fx: 270, fy: 240, cx: 162, cy: 117,
    distortion: {...first.distortion, k1: 0.3, p1: -0.012, p2: 0.017}}
  return JSON.stringify({format: FORMAT, version: 1, coordinateSystem: 'oriented-pixel-centers', groups: [first, second]}, null, 2) + '\n'
}
