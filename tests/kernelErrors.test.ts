import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { APP_STRINGS, translate } from '../src/i18n/appStrings'
import {
  DirectInputError,
  KERNEL_ERROR_CODES,
  KERNEL_ERROR_REASONS,
  KERNEL_ERROR_STEPS,
  kernelErrorSpec,
  LANGUAGE_ERROR_REASONS,
  languageErrorReason,
  presentKernelError,
  presentLanguageError,
} from '../src/i18n/kernelErrors'

const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map(m => m[1]).sort()

function rustSources(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name)
    if (statSync(path).isDirectory()) {
      if (!['tests', 'examples', 'benches', 'target'].includes(name)) rustSources(path, out)
    } else if (name.endsWith('.rs')) out.push(path)
  }
  return out
}

function crateErrorCodes(): string[] {
  const codes = new Set<string>()
  for (const crate of readdirSync('crates')) {
    const src = join('crates', crate, 'src')
    try {
      statSync(src)
    } catch {
      continue
    }
    for (const file of rustSources(src))
      for (const match of readFileSync(file, 'utf8').matchAll(/Error::new\("([A-Z][A-Z0-9_]+)"/g)) codes.add(match[1])
  }
  return [...codes].sort()
}

describe('kernel error presentation', () => {
  it('registers every error code raised by the Rust crates', () => {
    const known = new Set<string>(KERNEL_ERROR_CODES)
    const crate = crateErrorCodes()
    expect(crate.length).toBeGreaterThan(100)
    expect(crate.filter(code => !known.has(code))).toEqual([])
  })

  it('has a Russian and an English reason and step with identical placeholders', () => {
    for (const key of [
      'kernelError.unknown',
      ...KERNEL_ERROR_REASONS.map(r => `kernelError.${r}`),
      ...KERNEL_ERROR_STEPS.map(s => `kernelStep.${s}`),
    ]) {
      const ru = APP_STRINGS.ru[key],
        en = APP_STRINGS.en[key]
      expect(ru, `ru ${key}`).toBeTruthy()
      expect(en, `en ${key}`).toBeTruthy()
      expect(placeholders(ru!), key).toEqual(placeholders(en!))
    }
  })

  it('maps every registered code to a localized message, an input and a next step', () => {
    // CAD_OPERATION is the worker label for uncoded throws and keeps their own text.
    for (const code of KERNEL_ERROR_CODES.filter(c => c !== 'CAD_OPERATION')) {
      const spec = kernelErrorSpec(code)
      expect(spec, code).toBeDefined()
      for (const lang of ['ru', 'en'] as const) {
        const presented = presentKernelError({ code, message: 'raw kernel text' }, lang)
        expect(presented.message, `${lang} ${code}`).not.toMatch(/kernelError\.|kernelStep\./)
        if (lang === 'ru') expect(presented.message, code).not.toContain('raw kernel text')
        expect(presented.step, `${lang} ${code}`).not.toBe('')
        if (lang === 'ru') expect(presented.message, code).toMatch(/[а-яё]/i)
      }
      if (spec!.input === 'field') expect(spec!.field, code).toBeTruthy()
    }
  })

  it('falls back to a generic localized message that keeps an unknown code', () => {
    const ru = presentKernelError({ code: 'BREP_SOMETHING_NEW', message: 'x' }, 'ru')
    expect(ru.message).toContain('BREP_SOMETHING_NEW')
    expect(ru.message).toMatch(/Ошибка вычисления/)
    expect(presentKernelError({ code: 'BREP_SOMETHING_NEW', message: 'x' }, 'en').message).toMatch(
      /Calculation error \(BREP_SOMETHING_NEW\)/,
    )
  })

  it('keeps the text of uncoded errors and of the worker catch-all code', () => {
    expect(presentKernelError(new Error('Заполните оба поля'), 'ru').message).toBe('Заполните оба поля')
    expect(presentKernelError({ code: 'CAD_OPERATION', message: 'own text' }, 'en').message).toBe('own text')
  })

  it('points parameter refusals at the inspector field and objects at their ids', () => {
    const height = presentKernelError(new DirectInputError('DIRECT_EXTRUDE_HEIGHT', 'x', 'height'), 'en')
    expect(height).toMatchObject({ input: 'field', field: 'height', code: 'DIRECT_EXTRUDE_HEIGHT' })
    expect(presentKernelError({ code: 'BREP_INVALID_ANGLE', message: '' }, 'ru').field).toBe('revolveAngle')
    const sketch = presentKernelError(
      new DirectInputError('DIRECT_SKETCH_SELF_INTERSECTION', 'x', undefined, ['bow']),
      'ru',
      { target: 'Эскиз 1' },
    )
    expect(sketch.targetIds).toEqual(['bow'])
    expect(sketch.message).toMatch(/^Эскиз 1: Контур эскиза пересекает сам себя\./)
  })

  it('keeps the English kernel text as detail only in the English UI', () => {
    const error = { code: 'NURBS_INVALID_INPUT', message: 'Curve endpoint is outside the span' }
    expect(presentKernelError(error, 'en').message).toMatch(/Details: Curve endpoint is outside the span$/)
    expect(presentKernelError(error, 'ru').message).not.toContain('Curve endpoint')
  })

  it('translates the exact B-rep shell refusal', () => {
    expect(presentKernelError({ code: 'BREP_ANALYTIC_SHELL_REFUSED', message: '' }, 'ru').message).toMatch(
      /оболочка.*не подтверждена/,
    )
  })
})

const FREQUENT_KERNEL_CODES = [
  'NURBS_INVALID_INPUT',
  'BREP_RESOURCE_LIMIT',
  'GEOMETRY_INVALID_INPUT',
  'BREP_INVALID_INPUT',
  'BREP_UNSUPPORTED_OPERATION',
  'BREP_PROFILE_CONTACTS',
  'NURBS_NUMERIC_ERROR',
  'NURBS_RESOURCE_LIMIT',
  'BREP_THREAD_PATCH_UNRESOLVED',
  'BREP_INVALID_THREAD_PATCH',
  'BREP_RATIONAL_SWEEP_REFUSED',
  'FRAME_INVALID_INPUT',
  'BREP_SOURCE_CONTACT',
  'BREP_VARIABLE_RADIUS_FILLET_REFUSED',
  'BREP_ANALYTIC_FILLET_REFUSED',
  'BREP_CERTIFIED_TESSELLATION_REFUSED',
  'BREP_EXACT_FILLET_REFUSED',
  'BREP_CERTIFIED_MASS_REFUSED',
  'BREP_NURBS_STEP_REFUSED',
  'BREP_ANALYSIS_INDETERMINATE',
  'BREP_VALENCE3_CORNER_BLEND_REFUSED',
  'BREP_OFFSET_CAP_REFUSED',
  'BREP_INVALID_TOPOLOGY',
] as const
const FAMILY_REASONS = new Set([
  'invalidInput',
  'refused',
  'unproven',
  'resourceLimit',
  'topology',
  'importFile',
  'failed',
])

describe('exact kernel messages and coded TypeScript refusals', () => {
  it('gives the most frequent kernel codes an exact, non-family message in both languages', () => {
    for (const code of FREQUENT_KERNEL_CODES) {
      const spec = kernelErrorSpec(code)!
      expect(FAMILY_REASONS.has(spec.reason), code).toBe(false)
      for (const lang of ['ru', 'en'] as const) {
        const message = presentKernelError({ code, message: 'kernel text' }, lang).message
        expect(message, `${lang} ${code}`).toContain(code)
        expect(message, `${lang} ${code}`).not.toMatch(/kernelError\.|kernelStep\./)
      }
    }
  })

  it('points fillet, chamfer, sweep, offset and pattern refusals at their inspector fields', () => {
    expect(kernelErrorSpec('BREP_EXACT_FILLET_REFUSED')).toMatchObject({ field: 'radius', step: 'reduceRadius' })
    expect(kernelErrorSpec('BREP_EXACT_CHAMFER_REFUSED')).toMatchObject({ field: 'radius', step: 'reduceChamfer' })
    expect(kernelErrorSpec('BREP_VARIABLE_RADIUS_FILLET_REFUSED')?.field).toBe('endRadius')
    expect(kernelErrorSpec('BREP_OFFSET_CAP_REFUSED')?.field).toBe('distance')
    expect(kernelErrorSpec('DIRECT_SWEEP_DEVIATION')?.field).toBe('sweepDeviation')
    expect(kernelErrorSpec('DIRECT_PATTERN_GRID')?.field).toBe('patternCount')
    expect(presentKernelError({ code: 'BREP_EXACT_FILLET_REFUSED', message: '' }, 'ru').message).toMatch(
      /^Скругление не подтверждено.*Уменьшите радиус/,
    )
  })

  it('keeps the English kernel text only for exact messages that stay broad', () => {
    expect(presentKernelError({ code: 'BREP_INVALID_INPUT', message: 'Face 3 is open' }, 'en').message).toMatch(
      /Details: Face 3 is open$/,
    )
    expect(presentKernelError({ code: 'BREP_EXACT_FILLET_REFUSED', message: 'raw' }, 'en').message).not.toContain('raw')
  })

  it('maps the formerly uncoded TypeScript refusals to codes, fields and objects', async () => {
    const { constructSolidSurface, buildSolidSurface } = await import('../src/kernel/solidSurfaceConstruction')
    const { setLiveBodyPattern, validateLivePatternInput } = await import('../src/app/liveBodyPattern')
    const { editSketchDimension } = await import('../src/app/directDimensions')
    const doc = { bodies: [], sketches: [], curves: [] } as never
    expect(() => constructSolidSurface(doc, ['c1', 'c1'], 'nurbs-loft', 's')).toThrow(
      expect.objectContaining({ code: 'DIRECT_CURVES_DISTINCT', targetIds: ['c1'] }),
    )
    expect(() => constructSolidSurface(doc, ['c1', 'c2'], 'nurbs-loft', 's')).toThrow(
      expect.objectContaining({ code: 'DIRECT_CURVE_MISSING' }),
    )
    expect(() => constructSolidSurface(doc, ['c1'], 'nurbs-sweep', 's')).toThrow(
      expect.objectContaining({ code: 'DIRECT_SURFACE_INPUTS', targetIds: ['c1'] }),
    )
    // The worker builder rethrows coded refusals so their field and target ids cross the boundary.
    expect(() =>
      buildSolidSurface(doc, {
        kind: 'nurbs-loft',
        ids: ['c1', 'c1'],
        id: 's',
        reversed: [],
        sweep: { mode: 'translation', normal: [0, 0, 1], sections: 4, maxDeviation: 1 },
      }),
    ).toThrow(expect.objectContaining({ code: 'DIRECT_CURVES_DISTINCT' }))
    const body = { id: 'b', instance: { sourceId: 'x' } }
    expect(() => setLiveBodyPattern({ bodies: [body], sketches: [] } as never, 'b', null)).toThrow(
      expect.objectContaining({ code: 'DIRECT_PATTERN_SOURCE', targetIds: ['b'] }),
    )
    expect(() => validateLivePatternInput({ kind: 'grid', rows: 1, columns: 1, spacing: [1, 1] }, 'b')).toThrow(
      expect.objectContaining({ code: 'DIRECT_PATTERN_GRID', field: 'patternCount', targetIds: ['b'] }),
    )
    expect(() => validateLivePatternInput({ kind: 'grid', rows: 2, columns: 2, spacing: [1, Infinity] }, 'b')).toThrow(
      expect.objectContaining({ code: 'DIRECT_PATTERN_GRID' }),
    )
    expect(() => validateLivePatternInput({ kind: 'grid', rows: 16, columns: 17, spacing: [1, 1] }, 'b')).toThrow(
      expect.objectContaining({ code: 'DIRECT_PATTERN_GRID' }),
    )
    expect(() => validateLivePatternInput({ kind: 'grid', rows: 16, columns: 16, spacing: [1, 1] }, 'b')).not.toThrow()
    expect(() => validateLivePatternInput({ kind: 'radial', count: 1, axis: 'z', center: [0, 0, 0] }, 'b')).toThrow(
      expect.objectContaining({ code: 'DIRECT_PATTERN_RADIAL' }),
    )
    const arc = {
      id: 'a',
      name: 'Arc',
      closed: false,
      points: [],
      analytic: { kind: 'arc', center: [0, 0], radius: 1, start: 0, sweep: 90 },
      dimensions: [],
    }
    expect(() => editSketchDimension(arc as never, 0, 2)).toThrow(
      expect.objectContaining({ code: 'DIRECT_DIMENSION_ANALYTIC' }),
    )
    for (const code of [
      'DIRECT_CURVES_DISTINCT',
      'DIRECT_CURVE_MISSING',
      'DIRECT_PATTERN_SOURCE',
      'DIRECT_PATTERN_GRID',
      'DIRECT_DIMENSION_ANALYTIC',
      'DIRECT_PROFILE_PLANE',
      'DIRECT_TARGET_BODY',
    ] as const) {
      const ru = presentKernelError(new DirectInputError(code, 'x'), 'ru').message
      expect(ru, code).toMatch(/[а-яё]/i)
      expect(ru, code).not.toContain('x')
    }
  })
})

describe('OpenSCAD language diagnostics', () => {
  const codes = [...readFileSync('src/core/languageContract.ts', 'utf8').matchAll(/'(E_[A-Z0-9_]+)'/g)].map(m => m[1]!)

  it('maps every language diagnostic code to a reason with RU and EN text', () => {
    expect(codes.length).toBeGreaterThan(30)
    for (const code of codes) {
      const reason = languageErrorReason(code)
      expect(reason, code).toBeDefined()
      expect(LANGUAGE_ERROR_REASONS).toContain(reason)
      expect(APP_STRINGS.ru[`languageError.${reason}`], code).toBeTruthy()
      expect(APP_STRINGS.en[`languageError.${reason}`], code).toBeTruthy()
    }
  })

  it('localizes coded build errors and keeps the evaluator text as detail', () => {
    const t = (key: string) => translate('ru', key)
    const text = presentLanguageError({ code: 'E_IMPORT_FILE_MISSING', message: 'Line 2, column 1: missing.stl' }, t)
    expect(text).toMatch(/^Файл не найден/)
    expect(text).toContain('(E_IMPORT_FILE_MISSING)')
    expect(text).toContain('missing.stl')
    expect(languageErrorReason('E_FEATURE_INCLUDE')).toBe('unsupportedFeature')
    expect(languageErrorReason('E_TEXT_GLYPH_LIMIT')).toBe('limit')
    expect(presentLanguageError({ code: 'BREP_FAILED', message: 'raw' }, t)).toBe('raw')
    expect(presentLanguageError({ message: 'Line 1, column 1: syntax' }, t)).toBe('Line 1, column 1: syntax')
  })
})
