import { OPENSCAD_TEXT_MAX_PATH_COMMANDS, OPENSCAD_TEXT_MAX_VERTICES } from '../openScadText'
import type { HarfBuzzApi, HarfBuzzBlob, HarfBuzzFace, HarfBuzzFont, HarfBuzzPathCommand, OpenScadTextDirection, OpenScadTextFont, OpenScadTextHorizontalAlignment, OpenScadTextPoint, OpenScadTextVerticalAlignment, RawContour } from '../openScadText'
import { OpenScadProject } from '../openScadProject'
import { textError } from './prepareOpenScadTextAssets'

export function normalizeDirection(value: string): OpenScadTextDirection | undefined {
  const normalized = value.toLocaleLowerCase('en-US')
  return normalized === 'ltr' || normalized === 'rtl' || normalized === 'ttb' || normalized === 'btt'
    ? normalized
    : undefined
}

export function detectedScript(text: string): string {
  for (const character of text) {
    const codePoint = character.codePointAt(0)!
    if ((codePoint >= 0x0590 && codePoint <= 0x05ff)
      || (codePoint >= 0xfb1d && codePoint <= 0xfb4f)) return 'Hebr'
    if ((codePoint >= 0x0600 && codePoint <= 0x08ff)
      || (codePoint >= 0xfb50 && codePoint <= 0xfdff)
      || (codePoint >= 0xfe70 && codePoint <= 0xfeff)) return 'Arab'
    if (codePoint >= 0x0900 && codePoint <= 0x097f) return 'Deva'
    if (codePoint >= 0x0980 && codePoint <= 0x09ff) return 'Beng'
    if (codePoint >= 0x0e00 && codePoint <= 0x0e7f) return 'Thai'
    if (codePoint >= 0x3040 && codePoint <= 0x309f) return 'Hira'
    if (codePoint >= 0x30a0 && codePoint <= 0x30ff) return 'Kana'
    if ((codePoint >= 0x3400 && codePoint <= 0x9fff)
      || (codePoint >= 0x20000 && codePoint <= 0x3134f)) return 'Hani'
    if ((codePoint >= 0xac00 && codePoint <= 0xd7af)
      || (codePoint >= 0x1100 && codePoint <= 0x11ff)) return 'Hang'
    if (codePoint >= 0x0370 && codePoint <= 0x03ff) return 'Grek'
    if (codePoint >= 0x0400 && codePoint <= 0x052f) return 'Cyrl'
    if ((codePoint >= 0x0041 && codePoint <= 0x005a)
      || (codePoint >= 0x0061 && codePoint <= 0x007a)
      || (codePoint >= 0x00c0 && codePoint <= 0x02af)) return 'Latn'
  }
  return 'Zyyy'
}

export function effectiveScript(text: string, requested: string): string {
  const trimmed = requested.trim()
  return /^[A-Za-z]{4,32}$/u.test(trimmed) ? trimmed : detectedScript(text)
}

export function directionForScript(script: string): OpenScadTextDirection {
  const tag = script.slice(0, 4).toLocaleLowerCase('en-US')
  return [
    'adlm', 'arab', 'hebr', 'mand', 'mani', 'mend', 'merc', 'mero', 'nkoo',
    'orkh', 'phli', 'phlp', 'phnx', 'prti', 'rohg', 'samr', 'sarb', 'sogd',
    'sogo', 'syrc', 'thaa', 'yezi',
  ].includes(tag) ? 'rtl' : 'ltr'
}

export function horizontalAlignment(value: string): OpenScadTextHorizontalAlignment {
  return value === 'center' || value === 'right' ? value : 'left'
}

export function verticalAlignment(value: string): OpenScadTextVerticalAlignment {
  return value === 'top' || value === 'center' || value === 'bottom' ? value : 'baseline'
}

export function samePoint(
  left: readonly [number, number],
  right: readonly [number, number],
): boolean {
  return left[0] === right[0] && left[1] === right[1]
}

export function flattenGlyph(
  commands: readonly HarfBuzzPathCommand[],
  curveSegments: number,
  glyphId: number,
): readonly RawContour[] {
  if (commands.length > OPENSCAD_TEXT_MAX_PATH_COMMANDS) {
    return textError(
      'E_TEXT_PATH_LIMIT',
      `text() glyph ${glyphId} exceeds ${OPENSCAD_TEXT_MAX_PATH_COMMANDS.toLocaleString('en-US')} path commands.`,
      { limit: OPENSCAD_TEXT_MAX_PATH_COMMANDS, actual: commands.length },
    )
  }
  const contours: RawContour[] = []
  let points: Array<readonly [number, number]> = []
  let pen: readonly [number, number] = [0, 0]
  let commandBudget = 0

  const finish = (): void => {
    while (points.length > 1 && samePoint(points[0], points.at(-1)!)) points.pop()
    if (points.length >= 3) contours.push(Object.freeze({ points: Object.freeze(points) }))
    points = []
  }
  const add = (x: number, y: number): void => {
    if (!Number.isFinite(x) || !Number.isFinite(y)) {
      return textError('E_TEXT_FONT_INVALID', `text() glyph ${glyphId} contains non-finite outline coordinates.`)
    }
    points.push(Object.freeze([x, y] as const))
    commandBudget++
    if (commandBudget > OPENSCAD_TEXT_MAX_VERTICES) {
      return textError(
        'E_TEXT_VERTEX_LIMIT',
        `text() glyph ${glyphId} exceeds ${OPENSCAD_TEXT_MAX_VERTICES.toLocaleString('en-US')} flattened vertices.`,
        { limit: OPENSCAD_TEXT_MAX_VERTICES, actual: commandBudget },
      )
    }
  }

  for (const command of commands) {
    const values = command.values
    switch (command.type) {
      case 'M': {
        if (values.length !== 2) return textError('E_TEXT_FONT_INVALID', `text() glyph ${glyphId} has an invalid move command.`)
        finish()
        pen = [values[0], values[1]]
        add(pen[0], pen[1])
        break
      }
      case 'L': {
        if (values.length !== 2) return textError('E_TEXT_FONT_INVALID', `text() glyph ${glyphId} has an invalid line command.`)
        pen = [values[0], values[1]]
        add(pen[0], pen[1])
        break
      }
      case 'Q': {
        if (values.length !== 4) return textError('E_TEXT_FONT_INVALID', `text() glyph ${glyphId} has an invalid quadratic command.`)
        const from = pen
        const control: readonly [number, number] = [values[0], values[1]]
        const to: readonly [number, number] = [values[2], values[3]]
        for (let step = 1; step <= curveSegments; step++) {
          const t = step / curveSegments
          const inverse = 1 - t
          add(
            from[0] * inverse * inverse + control[0] * 2 * inverse * t + to[0] * t * t,
            from[1] * inverse * inverse + control[1] * 2 * inverse * t + to[1] * t * t,
          )
        }
        pen = to
        break
      }
      case 'C': {
        if (values.length !== 6) return textError('E_TEXT_FONT_INVALID', `text() glyph ${glyphId} has an invalid cubic command.`)
        const from = pen
        const control1: readonly [number, number] = [values[0], values[1]]
        const control2: readonly [number, number] = [values[2], values[3]]
        const to: readonly [number, number] = [values[4], values[5]]
        for (let step = 1; step <= curveSegments; step++) {
          const t = step / curveSegments
          const inverse = 1 - t
          add(
            from[0] * inverse ** 3 + control1[0] * 3 * inverse ** 2 * t
              + control2[0] * 3 * inverse * t ** 2 + to[0] * t ** 3,
            from[1] * inverse ** 3 + control1[1] * 3 * inverse ** 2 * t
              + control2[1] * 3 * inverse * t ** 2 + to[1] * t ** 3,
          )
        }
        pen = to
        break
      }
      case 'Z':
        if (values.length !== 0) return textError('E_TEXT_FONT_INVALID', `text() glyph ${glyphId} has an invalid close command.`)
        finish()
        break
    }
  }
  finish()
  return Object.freeze(contours)
}

export function freezePoint(x: number, y: number): OpenScadTextPoint {
  return Object.freeze([x, y] as const)
}

export function fontForLayout(
  hb: HarfBuzzApi,
  project: OpenScadProject,
  descriptor: OpenScadTextFont,
): { blob: HarfBuzzBlob; face: HarfBuzzFace; font: HarfBuzzFont } {
  const file = project.read(descriptor.path)
  if (file?.kind !== 'blob' || file.sha256 !== descriptor.sha256) {
    return textError(
      'E_TEXT_PROJECT_MISMATCH',
      `Prepared text font ${descriptor.path} no longer matches the project snapshot.`,
      { fontPath: descriptor.path, faceIndex: descriptor.faceIndex },
    )
  }
  const blob = hb.createBlob(file.data)
  let face: HarfBuzzFace | undefined
  let font: HarfBuzzFont | undefined
  try {
    face = hb.createFace(blob, descriptor.faceIndex)
    if (face.upem !== descriptor.unitsPerEm) throw new Error('units-per-em changed')
    font = hb.createFont(face)
    font.setScale(face.upem, face.upem)
    return { blob, face, font }
  } catch (error) {
    font?.destroy()
    face?.destroy()
    blob.destroy()
    return textError(
      'E_TEXT_FONT_INVALID',
      `Font ${descriptor.path} could not create a shaping face.`,
      { fontPath: descriptor.path, faceIndex: descriptor.faceIndex },
      error,
    )
  }
}
