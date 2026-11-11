import type { DirectDocument } from '../../../app/directModeling'
import { finite } from '../../mainSolidProtocol'
import { isSourceBodyRecordPayload } from '../../sourceBodyArchive'
import { arrayOf } from '../mainSolidExpectation'

export const isObjectRecord = (value: unknown): value is Record<string, unknown> =>
  !!value && typeof value === 'object'

export const isSafeNonnegativeCount = (value: unknown): value is number =>
  Number.isSafeInteger(value) && Number(value) >= 0

export const isSafeIntegerValue = (value: unknown): value is number =>
  Number.isSafeInteger(value)

export const isNonnegativeFinite = (value: unknown): value is number =>
  finite(value) && value >= 0

export const isOrderedInterval = (value: unknown): value is [number, number] =>
  arrayOf(value, 2, finite) && (value as number[])[0] <= (value as number[])[1]

export function validDirectDocument(value: unknown): value is DirectDocument {
  if (!isObjectRecord(value)) return false
  const doc = value as unknown as DirectDocument
  return (
    doc.version === 1 &&
    Array.isArray(doc.sketches) &&
    Array.isArray(doc.bodies) &&
    (doc.sourceBodies === undefined ||
      (Array.isArray(doc.sourceBodies) &&
        doc.sourceBodies.length <= 200 &&
        doc.sourceBodies.every(isSourceBodyRecordPayload)))
  )
}
