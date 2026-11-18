import { describe, it, expect } from 'vitest'
import { extrudeDirectSketch } from '../src/app/directModeling'
import {
  DOCUMENT_LENGTH_UNITS,
  formatDimensionValue,
  parseDocumentLengthUnit,
  stepLengthUnit,
  stlExportPositionsFromMillimetres,
  stlImportPositionsToMillimetres,
} from '../src/app/documentUnits'
import { exportFacetedStep, importFacetedStep } from '../src/kernel/cadStep'

const box = () =>
  extrudeDirectSketch(
    {
      id: 's',
      name: 'Box',
      closed: true,
      points: [
        [0, 0],
        [12.7, 0],
        [12.7, 25.4],
        [0, 25.4],
      ],
    },
    10,
    'b',
  )

const extent = (positions: ArrayLike<number>) => {
  const xs = Array.from(positions).filter((_, i) => i % 3 === 0)
  return Math.max(...xs) - Math.min(...xs)
}

describe('N5 document units at import and export', () => {
  it('writes STL coordinates in the document unit and reads them back into millimetres', () => {
    const mm = Float64Array.from([0, 0, 0, 25.4, 0, 0, 0, 254, 1000])
    const inch = stlExportPositionsFromMillimetres(mm, 'in')
    expect(Array.from(inch)).toEqual([0, 0, 0, 1, 0, 0, 0, 10, expect.closeTo(39.37007874015748, 12)])
    expect(Array.from(stlExportPositionsFromMillimetres(mm, 'cm'))).toEqual([0, 0, 0, 2.54, 0, 0, 0, 25.4, 100])
    expect(Array.from(stlExportPositionsFromMillimetres(mm, 'm'))[8]).toBe(1)
    expect(stlImportPositionsToMillimetres([1, 2.5], 'in')).toEqual(Float64Array.from([25.4, 63.5]))
    for (const unit of DOCUMENT_LENGTH_UNITS) {
      const back = stlImportPositionsToMillimetres(stlExportPositionsFromMillimetres(mm, unit), unit)
      back.forEach((v, i) => expect(v).toBeCloseTo(mm[i]!, 10))
    }
    // Millimetre documents keep the bytes unchanged.
    expect(stlExportPositionsFromMillimetres(mm, 'mm')).toEqual(mm)
  })

  it('exports faceted STEP in each document unit and imports it back to the same millimetres', () => {
    const body = box()
    for (const unit of DOCUMENT_LENGTH_UNITS) {
      const text = exportFacetedStep([body], unit)
      expect(stepLengthUnit(text)).toBe(unit)
      const [imported] = importFacetedStep(text)
      expect(extent(imported!.mesh.positions)).toBeCloseTo(12.7, 9)
    }
    const inchText = exportFacetedStep([body], 'in')
    expect(inchText).toContain("CONVERSION_BASED_UNIT('INCH'")
    expect(inchText).toMatch(/CARTESIAN_POINT\('',\(0\.5,/)
    expect(exportFacetedStep([body], 'cm')).toContain('SI_UNIT(.CENTI.,.METRE.)')
  })

  it('refuses unknown units clearly', () => {
    expect(() => parseDocumentLengthUnit('ft')).toThrow(/Unknown document length unit "ft".*mm, cm, m, in/)
    expect(() => parseDocumentLengthUnit(undefined)).toThrow(/Unknown document length unit/)
    const text = exportFacetedStep([box()], 'mm')
    expect(() => importFacetedStep(text.replace('.MILLI.', '.MICRO.'))).toThrow(
      /Unknown STEP length unit prefix \.MICRO\./,
    )
    expect(() => importFacetedStep(exportFacetedStep([box()], 'in').replace("'INCH'", "'FOOT'"))).toThrow(
      /Unknown STEP length unit 'FOOT'/,
    )
    expect(() =>
      importFacetedStep(exportFacetedStep([box()], 'in').replace('LENGTH_MEASURE(25.4)', 'LENGTH_MEASURE(25.)')),
    ).toThrow(/INCH unit must be defined as 25.4 mm/)
  })

  it('formats dimension values in the document unit', () => {
    expect(formatDimensionValue(25.4, 'in')).toBe('1.00 in')
    expect(formatDimensionValue(1234, 'm')).toBe('1.23 m')
    expect(formatDimensionValue(45, 'in', true)).toBe('45.00°')
    expect(formatDimensionValue(null, 'cm')).toBe('—')
  })
})
