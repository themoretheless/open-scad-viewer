import { OPENSCAD_2021_TEXT_EM_SCALE, OPENSCAD_TEXT_MAX_ABS_SIZE, OPENSCAD_TEXT_MAX_ABS_SPACING, OPENSCAD_TEXT_MAX_CODE_POINTS, OPENSCAD_TEXT_MAX_GLYPHS, OPENSCAD_TEXT_MAX_VERTICES, OpenScadTextError, preparedRuntimes } from '../openScadText'
import type { OpenScadTextContour, OpenScadTextGlyph, OpenScadTextLayout, OpenScadTextParameters, PreparedOpenScadTextAssets, RawContour } from '../openScadText'
import { OpenScadProject } from '../openScadProject'
import { boundedParameter, calculateOpenScadTextSegments, codePointCount, resolveFont, textError } from './prepareOpenScadTextAssets'
import { directionForScript, effectiveScript, flattenGlyph, fontForLayout, freezePoint, horizontalAlignment, normalizeDirection, verticalAlignment } from './normalizeDirection'
import { isWellFormedUnicode } from '../../core/unicode'

/**
 * Shape Unicode and emit real per-glyph outline polygons. This function is
 * synchronous after prepareOpenScadTextAssets(), matching the geometry evaluator.
 */
export function renderOpenScadText(
  project: OpenScadProject,
  prepared: PreparedOpenScadTextAssets,
  parameters: OpenScadTextParameters = {},
  sourcePath = project.entrypoint,
): OpenScadTextLayout {
  if (!(project instanceof OpenScadProject)) {
    return textError('E_TEXT_PROJECT_REQUIRED', 'text() requires an immutable OpenScadProject VFS.')
  }
  if (prepared.projectSha256 !== project.sha256) {
    return textError(
      'E_TEXT_PROJECT_MISMATCH',
      'Prepared text assets belong to a different OpenScadProject snapshot.',
      { sourcePath },
    )
  }
  const text = parameters.text ?? ''
  const fontQuery = parameters.font ?? ''
  if (typeof text !== 'string' || !isWellFormedUnicode(text) || text.includes('\u0000')) {
    return textError('E_TEXT_PARAMETER_INVALID', 'text() text must be a well-formed Unicode string without NUL.')
  }
  if (typeof fontQuery !== 'string' || !isWellFormedUnicode(fontQuery)) {
    return textError('E_TEXT_PARAMETER_INVALID', 'text() font must be a well-formed Unicode string.')
  }
  const points = codePointCount(text)
  if (points > OPENSCAD_TEXT_MAX_CODE_POINTS) {
    return textError(
      'E_TEXT_PARAMETER_LIMIT',
      `text() exceeds ${OPENSCAD_TEXT_MAX_CODE_POINTS.toLocaleString('en-US')} Unicode code points.`,
      { limit: OPENSCAD_TEXT_MAX_CODE_POINTS, actual: points },
    )
  }
  const size = boundedParameter(parameters.size ?? 10, 'size', OPENSCAD_TEXT_MAX_ABS_SIZE)
  const spacing = boundedParameter(parameters.spacing ?? 1, 'spacing', OPENSCAD_TEXT_MAX_ABS_SPACING)
  const language = parameters.language ?? 'en'
  const requestedDirection = parameters.direction ?? ''
  const requestedScript = parameters.script ?? ''
  if (typeof language !== 'string' || !/^[A-Za-z0-9-]{1,64}$/u.test(language)) {
    return textError('E_TEXT_PARAMETER_INVALID', 'text() language must be a bounded ASCII BCP-47 tag.')
  }
  if (typeof requestedDirection !== 'string' || typeof requestedScript !== 'string'
    || !isWellFormedUnicode(requestedDirection) || !isWellFormedUnicode(requestedScript)) {
    return textError('E_TEXT_PARAMETER_INVALID', 'text() direction and script must be Unicode strings.')
  }
  const curveSegments = calculateOpenScadTextSegments(
    size,
    parameters.$fn ?? 0,
    parameters.$fs ?? 2,
    parameters.$fa ?? 12,
  )
  const descriptor = resolveFont(project, prepared, sourcePath, fontQuery)
  const hb = preparedRuntimes.get(prepared)
  if (!hb) return textError('E_TEXT_RUNTIME_UNAVAILABLE', 'Prepared text assets have no shaping runtime.')

  const script = effectiveScript(text, requestedScript)
  const explicitDirection = normalizeDirection(requestedDirection)
  const direction = explicitDirection ?? directionForScript(script)
  const halign = horizontalAlignment(parameters.halign ?? 'left')
  const valign = verticalAlignment(parameters.valign ?? 'baseline')
  const unitScale = OPENSCAD_2021_TEXT_EM_SCALE / descriptor.unitsPerEm
  const { blob, face, font } = fontForLayout(hb, project, descriptor)
  const buffer = hb.createBuffer()
  try {
    buffer.addText(text)
    buffer.guessSegmentProperties()
    // OpenSCAD resolves direction from the effective script before shaping,
    // including when `script` was authored but `direction` was omitted.
    // Applying both resolved properties also keeps the returned layout and
    // HarfBuzz glyph order consistent for that case.
    buffer.setDirection(direction)
    if (script !== 'Zyyy') buffer.setScript(script)
    buffer.setLanguage(language)
    hb.shape(font, buffer)
    const shaped = buffer.json()
    if (shaped.length > OPENSCAD_TEXT_MAX_GLYPHS) {
      return textError(
        'E_TEXT_GLYPH_LIMIT',
        `text() shaping exceeds ${OPENSCAD_TEXT_MAX_GLYPHS.toLocaleString('en-US')} glyphs.`,
        { limit: OPENSCAD_TEXT_MAX_GLYPHS, actual: shaped.length },
      )
    }
    if (shaped.some(glyph => !Number.isSafeInteger(glyph.g) || glyph.g < 0
      || !Number.isSafeInteger(glyph.cl) || glyph.cl < 0
      || ![glyph.dx, glyph.dy, glyph.ax, glyph.ay].every(Number.isFinite))) {
      return textError('E_TEXT_SHAPING_FAILED', `Font ${descriptor.path} returned invalid shaping positions.`, {
        fontPath: descriptor.path,
        faceIndex: descriptor.faceIndex,
      })
    }

    const horizontal = direction === 'ltr' || direction === 'rtl'
    let width = 0
    let ascent = 0
    let descent = 0
    for (const glyph of shaped) {
      const extents = font.glyphExtents(glyph.g)
      if (horizontal) {
        const yMaximum = extents?.yBearing ?? 0
        const yMinimum = extents === null ? 0 : extents.yBearing + extents.height
        ascent = Math.max(ascent, Math.max(0, yMaximum * unitScale))
        descent = Math.max(descent, Math.max(0, -yMinimum * unitScale))
        width += glyph.ax * unitScale * spacing
      } else {
        const glyphWidth = extents === null ? 0 : Math.abs(extents.width) * unitScale
        width = Math.max(width, glyphWidth)
        ascent += glyph.ay * unitScale * spacing
      }
    }
    const xAlignment = halign === 'right' ? -width : halign === 'center' ? -width / 2 : 0
    const yAlignment = valign === 'top' ? -ascent
      : valign === 'center' ? descent / 2 - ascent / 2
        : valign === 'bottom' ? descent : 0

    const outlineCache = new Map<number, readonly RawContour[]>()
    const glyphs: OpenScadTextGlyph[] = []
    let advanceX = 0
    let advanceY = 0
    let polygonCount = 0
    let vertexCount = 0
    let minX = Infinity
    let minY = Infinity
    let maxX = -Infinity
    let maxY = -Infinity
    for (const glyph of shaped) {
      let rawContours = outlineCache.get(glyph.g)
      if (!rawContours) {
        rawContours = flattenGlyph(font.glyphToJson(glyph.g), curveSegments, glyph.g)
        outlineCache.set(glyph.g, rawContours)
      }
      const glyphOffsetX = xAlignment + glyph.dx * unitScale + advanceX
      const glyphOffsetY = yAlignment + glyph.dy * unitScale + advanceY
      const contours: OpenScadTextContour[] = []
      if (size !== 0) {
        for (const rawContour of rawContours) {
          const contour = rawContour.points.map(point => {
            const x = size * (point[0] * unitScale + glyphOffsetX)
            const y = size * (point[1] * unitScale + glyphOffsetY)
            if (!Number.isFinite(x) || !Number.isFinite(y)) {
              return textError('E_TEXT_PARAMETER_LIMIT', 'text() produced out-of-range outline coordinates.')
            }
            minX = Math.min(minX, x)
            minY = Math.min(minY, y)
            maxX = Math.max(maxX, x)
            maxY = Math.max(maxY, y)
            return freezePoint(x, y)
          })
          vertexCount += contour.length
          if (vertexCount > OPENSCAD_TEXT_MAX_VERTICES) {
            return textError(
              'E_TEXT_VERTEX_LIMIT',
              `text() exceeds ${OPENSCAD_TEXT_MAX_VERTICES.toLocaleString('en-US')} outline vertices.`,
              { limit: OPENSCAD_TEXT_MAX_VERTICES, actual: vertexCount },
            )
          }
          contours.push(Object.freeze(contour))
        }
      }
      polygonCount += contours.length
      glyphs.push(Object.freeze({
        glyphId: glyph.g,
        cluster: glyph.cl,
        flags: glyph.flags,
        offset: freezePoint(size * glyphOffsetX, size * glyphOffsetY),
        advance: freezePoint(size * glyph.ax * unitScale * spacing, size * glyph.ay * unitScale * spacing),
        contours: Object.freeze(contours),
      }))
      advanceX += glyph.ax * unitScale * spacing
      advanceY += glyph.ay * unitScale * spacing
    }

    return Object.freeze({
      font: descriptor,
      direction,
      script,
      language,
      halign,
      valign,
      spacing,
      size,
      curveSegments,
      glyphs: Object.freeze(glyphs),
      advance: freezePoint(size * advanceX, size * advanceY),
      width: size * width,
      ascent: size * ascent,
      descent: size * descent,
      bounds: vertexCount === 0 ? null : Object.freeze({
        min: freezePoint(minX, minY),
        max: freezePoint(maxX, maxY),
      }),
      polygonCount,
      vertexCount,
    })
  } catch (error) {
    if (error instanceof OpenScadTextError) throw error
    return textError(
      'E_TEXT_SHAPING_FAILED',
      `text() shaping failed for ${descriptor.path}: ${error instanceof Error ? error.message : String(error)}`,
      { fontPath: descriptor.path, faceIndex: descriptor.faceIndex },
      error,
    )
  } finally {
    buffer.destroy()
    font.destroy()
    face.destroy()
    blob.destroy()
  }
}
