export const escapeDrawingText = (s: string) =>
  s.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;' })[c]!)

export interface DrawingSheetFormatSpec {
  id: 'A4' | 'A3' | 'A2' | 'A1' | 'A0'
  widthMm: number
  heightMm: number
  marginLeftMm: number
  marginOtherMm: number
}

export const DRAWING_SHEET_FORMATS: Readonly<Record<'A4' | 'A3' | 'A2' | 'A1' | 'A0', DrawingSheetFormatSpec>> = {
  A4: { id: 'A4', widthMm: 297, heightMm: 210, marginLeftMm: 20, marginOtherMm: 5 },
  A3: { id: 'A3', widthMm: 420, heightMm: 297, marginLeftMm: 20, marginOtherMm: 5 },
  A2: { id: 'A2', widthMm: 594, heightMm: 420, marginLeftMm: 20, marginOtherMm: 10 },
  A1: { id: 'A1', widthMm: 841, heightMm: 594, marginLeftMm: 20, marginOtherMm: 10 },
  A0: { id: 'A0', widthMm: 1189, heightMm: 841, marginLeftMm: 20, marginOtherMm: 10 },
}

export interface DrawingToleranceCalloutOptions {
  /** Prefix symbol such as '⌀', 'R', or '□'. */
  prefix?: string
  /** Symmetric bilateral tolerance (±value in mm). */
  symmetricToleranceMm?: number
  /** Upper deviation in mm (for asymmetric limits). */
  upperDeviationMm?: number
  /** Lower deviation in mm (for asymmetric limits). */
  lowerDeviationMm?: number
  /** ISO fit class code (e.g. 'H7', 'g6', 'H7/g6'). */
  fitCode?: string
  /** Number of decimal places for nominal value (default: 2). */
  decimals?: number
  /** Unit suffix (default: 'mm', pass '' to omit). */
  unit?: string
}

/**
 * Formats an engineering drawing dimension with optional ISO/GD&T tolerance callouts
 * (symmetric `±`, bilateral `+upper/-lower`, fit class `H7/g6`, and diameter/radius prefix).
 */
export function formatDrawingDimensionWithTolerance(
  nominalMm: number,
  options: DrawingToleranceCalloutOptions = {},
): string {
  if (!Number.isFinite(nominalMm)) {
    throw new RangeError('Nominal dimension value must be finite.')
  }
  const decimals = options.decimals ?? 2
  const prefix = options.prefix ?? ''
  const unit = options.unit ?? 'mm'
  const base = `${prefix}${nominalMm.toFixed(decimals)}`
  const parts: string[] = [base]

  if (options.fitCode && options.fitCode.trim()) {
    parts.push(options.fitCode.trim())
  }

  if (options.symmetricToleranceMm !== undefined) {
    if (!Number.isFinite(options.symmetricToleranceMm) || options.symmetricToleranceMm < 0) {
      throw new RangeError('Symmetric tolerance must be a non-negative finite number.')
    }
    parts.push(`±${options.symmetricToleranceMm.toFixed(decimals)}`)
  } else if (options.upperDeviationMm !== undefined || options.lowerDeviationMm !== undefined) {
    const upper = options.upperDeviationMm ?? 0
    const lower = options.lowerDeviationMm ?? 0
    if (!Number.isFinite(upper) || !Number.isFinite(lower)) {
      throw new RangeError('Tolerance deviations must be finite numbers.')
    }
    const fmtDev = (v: number) => `${v >= 0 ? '+' : ''}${v.toFixed(decimals)}`
    parts.push(`(${fmtDev(upper)}/${fmtDev(lower)})`)
  }

  if (unit) {
    parts.push(unit)
  }
  return parts.join(' ')
}

export interface DrawingTitleBlockSvgOptions {
  format: keyof typeof DRAWING_SHEET_FORMATS
  title: string
  partNumber?: string
  material?: string
  scale?: string
  author?: string
  dateIso?: string
}

/**
 * Generates a standalone vector SVG engineering drawing sheet frame with outer sheet border,
 * inner GOST/ISO working border, and bottom-right 185×55 mm title block stamp.
 */
export function generateDrawingTitleBlockSvg(options: DrawingTitleBlockSvgOptions): string {
  const spec = DRAWING_SHEET_FORMATS[options.format]
  if (!spec) {
    throw new RangeError(`Unsupported drawing sheet format: ${String(options.format)}`)
  }
  const { widthMm, heightMm, marginLeftMm, marginOtherMm } = spec
  const innerX = marginLeftMm
  const innerY = marginOtherMm
  const innerW = widthMm - marginLeftMm - marginOtherMm
  const innerH = heightMm - marginOtherMm * 2

  const stampW = Math.min(185, innerW)
  const stampH = 55
  const stampX = innerX + innerW - stampW
  const stampY = innerY + innerH - stampH

  const title = escapeDrawingText(options.title.trim() || 'Untitled Part')
  const partNumber = escapeDrawingText((options.partNumber ?? 'N/A').trim() || 'N/A')
  const material = escapeDrawingText((options.material ?? 'Unspecified').trim() || 'Unspecified')
  const scale = escapeDrawingText((options.scale ?? '1:1').trim() || '1:1')
  const author = escapeDrawingText((options.author ?? 'Engineer').trim() || 'Engineer')
  const dateIso = escapeDrawingText((options.dateIso ?? new Date().toISOString().slice(0, 10)).trim())

  const col1X = stampX + 65
  const col2X = stampX + 135
  const row1Y = stampY + 15
  const row2Y = stampY + 35

  return [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${widthMm}mm" height="${heightMm}mm" viewBox="0 0 ${widthMm} ${heightMm}">`,
    `<rect width="${widthMm}" height="${heightMm}" fill="white"/>`,
    `<g font-family="sans-serif" fill="black" stroke="black">`,
    `<rect data-role="sheet-frame" x="${innerX}" y="${innerY}" width="${innerW}" height="${innerH}" fill="none" stroke-width="0.5"/>`,
    `<g data-role="title-block" transform="translate(0,0)">`,
    `<rect x="${stampX}" y="${stampY}" width="${stampW}" height="${stampH}" fill="none" stroke-width="0.5"/>`,
    `<path d="M${stampX} ${row1Y}H${stampX + stampW} M${stampX} ${row2Y}H${stampX + stampW} M${col1X} ${stampY}V${stampY + stampH} M${col2X} ${stampY}V${stampY + stampH}" fill="none" stroke-width="0.25"/>`,
    `<text x="${stampX + 3}" y="${stampY + 5}" font-size="2.2" stroke="none">Part No.</text>`,
    `<text x="${stampX + 3}" y="${stampY + 11}" font-size="3.2" font-weight="bold" stroke="none">${partNumber}</text>`,
    `<text x="${col1X + 3}" y="${stampY + 5}" font-size="2.2" stroke="none">Title</text>`,
    `<text x="${col1X + 3}" y="${stampY + 11}" font-size="3.2" font-weight="bold" stroke="none">${title}</text>`,
    `<text x="${col2X + 3}" y="${stampY + 5}" font-size="2.2" stroke="none">Format</text>`,
    `<text x="${col2X + 3}" y="${stampY + 11}" font-size="3.0" stroke="none">${spec.id}</text>`,
    `<text x="${stampX + 3}" y="${row1Y + 5}" font-size="2.2" stroke="none">Author</text>`,
    `<text x="${stampX + 3}" y="${row1Y + 12}" font-size="2.8" stroke="none">${author}</text>`,
    `<text x="${col1X + 3}" y="${row1Y + 5}" font-size="2.2" stroke="none">Material</text>`,
    `<text x="${col1X + 3}" y="${row1Y + 12}" font-size="2.8" stroke="none">${material}</text>`,
    `<text x="${col2X + 3}" y="${row1Y + 5}" font-size="2.2" stroke="none">Scale</text>`,
    `<text x="${col2X + 3}" y="${row1Y + 12}" font-size="2.8" stroke="none">${scale}</text>`,
    `<text x="${stampX + 3}" y="${row2Y + 6}" font-size="2.2" stroke="none">Date</text>`,
    `<text x="${stampX + 3}" y="${row2Y + 13}" font-size="2.8" stroke="none">${dateIso}</text>`,
    `<text x="${col1X + 3}" y="${row2Y + 6}" font-size="2.2" stroke="none">Standard</text>`,
    `<text x="${col1X + 3}" y="${row2Y + 13}" font-size="2.8" stroke="none">ISO 7200 / GOST 2.104</text>`,
    `<text x="${col2X + 3}" y="${row2Y + 6}" font-size="2.2" stroke="none">Sheet</text>`,
    `<text x="${col2X + 3}" y="${row2Y + 13}" font-size="2.8" stroke="none">1 / 1</text>`,
    `</g>`,
    `</g>`,
    `</svg>`,
  ].join('')
}
