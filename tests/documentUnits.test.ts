import { describe, it, expect } from 'vitest'
import { DOCUMENT_LENGTH_UNITS, MM_PER_UNIT, documentUnit, exportScaleFromMillimetres, formatLength, fromMillimetres, importScaleToMillimetres, isDocumentLengthUnit, toMillimetres } from '../src/app/documentUnits'
import { emptyDirectDocument, parseDirectDocument, serializeDirectDocument } from '../src/app/directModeling'
import { appendDirectCheckpoint, appendDirectDocumentEdit, executeDirectTransaction } from '../src/app/directTransactions'

describe('document length units', () => {
  it('uses exact conversion factors', () => {
    expect(MM_PER_UNIT).toEqual({ mm: 1, cm: 10, m: 1000, in: 25.4 })
    expect(toMillimetres(2, 'in')).toBe(50.8)
    expect(fromMillimetres(25.4, 'in')).toBe(1)
    expect(fromMillimetres(1500, 'm')).toBe(1.5)
    expect(toMillimetres(3, 'mm')).toBe(3)
    for (const unit of DOCUMENT_LENGTH_UNITS) {
      expect(importScaleToMillimetres(unit) * exportScaleFromMillimetres(unit)).toBeCloseTo(1, 15)
      expect(toMillimetres(fromMillimetres(123.456, unit), unit)).toBeCloseTo(123.456, 12)
    }
    expect(formatLength(254, 'in')).toBe('10.00 in')
    expect(isDocumentLengthUnit('ft')).toBe(false)
  })

  it('defaults legacy documents to millimetres and omits absent units', () => {
    const legacy = emptyDirectDocument()
    const text = serializeDirectDocument(legacy)
    expect(text).not.toContain('units')
    const parsed = parseDirectDocument(text)
    expect('units' in parsed).toBe(false)
    expect(documentUnit(parsed)).toBe('mm')
  })

  it('stores, validates and round-trips the document unit', () => {
    const doc = emptyDirectDocument(); doc.units = 'in'
    const parsed = parseDirectDocument(serializeDirectDocument(doc))
    expect(parsed.units).toBe('in')
    expect(documentUnit(parsed)).toBe('in')
    expect(() => parseDirectDocument(JSON.stringify({ ...emptyDirectDocument(), units: 'ft' }))).toThrow(/length unit/)
    expect(() => parseDirectDocument(JSON.stringify({ ...emptyDirectDocument(), units: 25.4 }))).toThrow(/length unit/)
  })

  it('records unit changes as replayable metadata edits', () => {
    const base = emptyDirectDocument()
    const inch = structuredClone(base); inch.units = 'in'
    const back = structuredClone(inch); delete back.units
    let script = appendDirectCheckpoint({ version: 1, actions: [] }, base)
    script = appendDirectDocumentEdit(script, base, inch)
    expect(executeDirectTransaction(base, JSON.stringify(script)).units).toBe('in')
    script = appendDirectDocumentEdit(script, inch, back)
    expect('units' in executeDirectTransaction(base, JSON.stringify(script))).toBe(false)
  })
})
