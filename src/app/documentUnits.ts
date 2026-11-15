/**
 * Document length units (roadmap N5). Kernel geometry is always millimetres;
 * the unit is document metadata applied at import/export and display.
 * Documents without `units` are millimetre documents.
 */
export const DOCUMENT_LENGTH_UNITS = ['mm', 'cm', 'm', 'in'] as const
export type DocumentLengthUnit = typeof DOCUMENT_LENGTH_UNITS[number]

/** Exact factors; the inch is 25.4 mm by definition. */
export const MM_PER_UNIT: Readonly<Record<DocumentLengthUnit, number>> = { mm: 1, cm: 10, m: 1000, in: 25.4 }

export const DEFAULT_DOCUMENT_UNIT: DocumentLengthUnit = 'mm'

export function isDocumentLengthUnit(value: unknown): value is DocumentLengthUnit {
  return typeof value === 'string' && (DOCUMENT_LENGTH_UNITS as readonly string[]).includes(value)
}

export function documentUnit(document: { units?: DocumentLengthUnit }): DocumentLengthUnit {
  return document.units ?? DEFAULT_DOCUMENT_UNIT
}

/** Display/export: millimetres to document units. */
export function fromMillimetres(valueMm: number, unit: DocumentLengthUnit): number {
  return unit === 'mm' ? valueMm : valueMm / MM_PER_UNIT[unit]
}

/** Import/input: document units to millimetres. */
export function toMillimetres(value: number, unit: DocumentLengthUnit): number {
  return unit === 'mm' ? value : value * MM_PER_UNIT[unit]
}

/** Coordinate factor for importing a file authored in `source` into kernel millimetres. */
export function importScaleToMillimetres(source: DocumentLengthUnit): number {
  return MM_PER_UNIT[source]
}

/** Coordinate factor for exporting kernel millimetres into a file in `target` units. */
export function exportScaleFromMillimetres(target: DocumentLengthUnit): number {
  return 1 / MM_PER_UNIT[target]
}

export function formatLength(valueMm: number, unit: DocumentLengthUnit, decimals = 2): string {
  return `${fromMillimetres(valueMm, unit).toFixed(decimals)} ${unit}`
}
