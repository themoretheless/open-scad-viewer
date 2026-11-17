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

describe('C4 OpenSCAD warnings and worker errors', () => {
  const sourceFiles = (dir: string, out: string[] = []): string[] => {
    for (const name of readdirSync(dir)) {
      const path = join(dir, name)
      if (statSync(path).isDirectory()) sourceFiles(path, out)
      else if (name.endsWith('.ts')) out.push(path)
    }
    return out
  }

  it('gives every emitted OPENSCAD_* warning code a pattern and a RU/EN reason with a next step', async () => {
    const { OPENSCAD_WARNING_ALIASES, OPENSCAD_WARNING_PATTERNS, OPENSCAD_WARNING_REASONS, openScadWarningReason } =
      await import('../src/i18n/kernelErrors')
    const emitted = new Set<string>()
    for (const file of [...sourceFiles('src/language/semantic'), ...sourceFiles('src/exchange/openScadImport')])
      for (const match of readFileSync(file, 'utf8').matchAll(/'(OPENSCAD_[A-Z_]+)'/g)) emitted.add(match[1]!)
    expect(emitted.size).toBeGreaterThanOrEqual(40)
    const covered = new Set<string>([
      ...OPENSCAD_WARNING_PATTERNS.map(([code]) => code),
      ...Object.keys(OPENSCAD_WARNING_ALIASES),
    ])
    expect([...emitted].filter(code => !covered.has(code))).toEqual([])
    for (const code of emitted) expect(openScadWarningReason(code), code).toBeDefined()
    for (const reason of OPENSCAD_WARNING_REASONS)
      for (const lang of ['ru', 'en'] as const) {
        const text = (APP_STRINGS[lang] as Record<string, string>)[`openScadWarning.${reason}`]
        expect(text, `${lang} ${reason}`).toBeTruthy()
        // A reason sentence plus a next-step sentence.
        expect(text!.split('. ').length, `${lang} ${reason}`).toBeGreaterThanOrEqual(2)
      }
  })

  it('recovers the code of every evaluator warning text and localizes it with the text as detail', async () => {
    const { openScadWarningCode, presentBuildWarning } = await import('../src/i18n/kernelErrors')
    const samples: Record<string, string[]> = {
      OPENSCAD_CHILDREN_BAD_INDEX: ['Non-numeric children index was ignored'],
      OPENSCAD_CHILDREN_BAD_SELECTION: [
        'Invalid children range was ignored',
        'children accepts an empty argument list, number, vector, or range',
      ],
      OPENSCAD_CHILDREN_INDEX_OUT_OF_BOUNDS: ['Children index -1 is outside 0..2'],
      OPENSCAD_CHILDREN_RANGE_LIMIT: ['children range exceeds the engine limit 10000'],
      OPENSCAD_COLOR_ALPHA_RANGE: ['The color vector alpha channel is outside the inclusive 0..1 range'],
      OPENSCAD_COLOR_CHANNEL_RANGE: ['Color channel 2 is outside the inclusive 0..1 range'],
      OPENSCAD_COLOR_UNKNOWN: ['Unknown OpenSCAD color name: blurple'],
      OPENSCAD_CYLINDER_AMBIGUOUS_RADII: ['cylinder combines a shared radius with an end-specific radius'],
      OPENSCAD_FA_TOO_SMALL: ['$fa was raised to the OpenSCAD minimum 0.01'],
      OPENSCAD_FS_TOO_SMALL: ['$fs was raised to the OpenSCAD minimum 0.01'],
      OPENSCAD_FN_NEGATIVE: ['Negative $fn was replaced with the automatic fragment mode'],
      OPENSCAD_FRAGMENTS_CLAMPED: ['Fragment count was clamped to the engine safety limit 48'],
      OPENSCAD_FRAGMENT_NON_FINITE: [
        'Non-finite $fa was replaced with its default',
        'Non-finite $fn selected the minimum fragment count',
        'Non-finite $fs was replaced with its default',
        'Non-finite fragment resolution was bounded by the engine safety limit',
      ],
      OPENSCAD_LINEAR_EXTRUDE_HEIGHT_DEFAULTED: ['Invalid linear_extrude height was replaced with 100'],
      OPENSCAD_LINEAR_EXTRUDE_SCALE_DEFAULTED: ['Invalid linear_extrude scale was replaced with [1, 1]'],
      OPENSCAD_LINEAR_EXTRUDE_SLICES_CLAMPED: ['linear_extrude slices were clamped to the engine limit 512'],
      OPENSCAD_POLYHEDRON_TRIANGLES_DEPRECATED: [
        'polyhedron triangles is a legacy alias; faces is the stable spelling',
      ],
      OPENSCAD_PRIMITIVE_INDEX_OUT_OF_BOUNDS: [
        'polygon skipped a path entry whose point index is outside the point vector',
        'polyhedron skipped a face entry whose point index is outside the point vector',
      ],
      OPENSCAD_PRIMITIVE_POINT_INVALID: [
        'polygon produced no outlines because a point failed exact vec2 conversion',
        'polyhedron stopped after a referenced point failed exact vec2/vec3 conversion',
      ],
      OPENSCAD_PRIMITIVE_RADIUS_SHADOWED: ['cylinder uses d1; the paired r1 value has no effect'],
      OPENSCAD_PRIMITIVE_RANGE_EMPTY: ['sphere parameters describe an empty object'],
      OPENSCAD_PRIMITIVE_SAFETY_REDUCTION: [
        'polyhedron was not expanded because its faces exceed the configured engine limit',
      ],
      OPENSCAD_PRIMITIVE_SIZE_DEFAULTED: [
        'cube size was not a scalar or exact 3-component numeric vector; unit size is used',
      ],
      OPENSCAD_RESIZE_BAD_NEWSIZE: ['resize newsize must be a vector'],
      OPENSCAD_RESIZE_ZERO_EXTENT: ['resize cannot map a zero-width axis z to a positive target'],
      OPENSCAD_ROTATE_AXIS_IGNORED: ['rotate ignores v when a is a vector'],
      OPENSCAD_ROTATE_EXTRUDE_ANGLE_DEFAULTED: ['Invalid rotate_extrude angle was replaced with 360'],
      OPENSCAD_ROTATE_PARTIAL_VECTOR: [
        'rotate retained its component-wise fallback matrix after a vector conversion problem',
      ],
      OPENSCAD_TRANSFORM_NONFINITE_EMPTY: ['scale produced a non-finite matrix, so its child geometry is removed'],
      OPENSCAD_TRANSFORM_PARAMETER_DEFAULTED: [
        'mirror uses its x-normal default because v is not an exact vec2 or vec3',
        'rotate replaced an invalid scalar angle or axis with its neutral/default value',
        'scale uses identity because v is not a scalar or exact vec2/vec3',
        'translate uses identity because v is not a finite exact vec2 or vec3',
      ],
      OPENSCAD_TRANSFORM_PARAMETER_RANGE: ['scale contains a zero or non-finite component'],
      OPENSCAD_DXF_QUERY_INVALID_ORIGIN: [
        'DXF query origin contains a non-finite coordinate; 2021.01 arithmetic is preserved.',
        'DXF query origin was not a two-number vector; [0, 0] is used.',
      ],
      OPENSCAD_DXF_QUERY_FILE_UNAVAILABLE: [
        'DXF query path could not be resolved: outside project',
        'DXF query asset a.dxf is missing from the project.',
      ],
      OPENSCAD_DXF_QUERY_NOT_PREPARED: ['DXF query asset a.dxf was not prepared as DXF.'],
      OPENSCAD_DXF_QUERY_INVALID_DATA: ['DXF query asset a.dxf could not be decoded: bad group code'],
      OPENSCAD_DXF_DIMENSION_NOT_FOUND: ['No matching DXF dimension was found in a.dxf.'],
      OPENSCAD_DXF_DIMENSION_TYPE_UNSUPPORTED: ['DXF dimension w uses unsupported type 7.'],
      OPENSCAD_DXF_CROSS_NOT_FOUND: ['The first two eligible DXF paths in a.dxf do not define a cross.'],
    }
    // The two rotate_extrude fragment aliases share these texts and are not separately recoverable.
    expect(Object.keys(samples)).toHaveLength(38)
    for (const [code, texts] of Object.entries(samples))
      for (const text of texts) {
        expect(openScadWarningCode(text), text).toBe(code)
        for (const lang of ['ru', 'en'] as const) {
          const line = presentBuildWarning(text, key => translate(lang, key))
          expect(line, `${lang} ${text}`).toContain(`(${code}) ${text}`)
          expect(line, `${lang} ${text}`).not.toMatch(/^openScadWarning\./)
        }
      }
    expect(presentBuildWarning('ECHO: "x"', key => translate('ru', key))).toBe('ECHO: "x"')
  })

  it('classifies the warnings the stable semantics actually emit', async () => {
    const { openScadWarningCode } = await import('../src/i18n/kernelErrors')
    const { resolveOpenScadColor } = await import('../src/language/semantic/openScadStableModuleSemantics')
    const seen: { code: string; message: string }[] = []
    const context = { warn: (warning: { code: string; message: string }) => seen.push(warning) }
    resolveOpenScadColor('blurple', undefined, context)
    resolveOpenScadColor([2, 0, 0], undefined, context)
    resolveOpenScadColor([0, 0, 0, 3], undefined, context)
    expect(new Set(seen.map(warning => warning.code)).size).toBe(3)
    for (const warning of seen) expect(openScadWarningCode(warning.message)).toBe(warning.code)
  })

  it('registers every geometry, exact-solid and MCP direct-geometry worker code with an exact message', () => {
    const supervisor = readFileSync('src/mcp/directGeometrySupervisor/errorsAndTypes.ts', 'utf8')
    const mcpCodes = [...new Set([...supervisor.matchAll(/'(E_DIRECT_GEOMETRY_[A-Z_]+)'/g)].map(match => match[1]!))]
    expect(mcpCodes).toHaveLength(6)
    const workerCodes = [
      ...mcpCodes,
      'GEOMETRY_PROTOCOL',
      'LANGUAGE_CONTRACT_UNSUPPORTED',
      'EXACT_SOLID_BODY_LIMIT',
      'EXACT_SOLID_DOCUMENT_LIMIT',
      'EXACT_SOLID_NOT_BREP',
      'WORKER_TIMEOUT',
      'WORKER_RESULT_UNPUBLISHABLE',
      'ENGINE_UNAVAILABLE',
      'CAPABILITY_UNAVAILABLE',
    ]
    for (const code of workerCodes) {
      expect(KERNEL_ERROR_CODES as readonly string[], code).toContain(code)
      const spec = kernelErrorSpec(code)!
      expect(FAMILY_REASONS.has(spec.reason), code).toBe(false)
      for (const lang of ['ru', 'en'] as const) {
        const message = presentKernelError({ code, message: 'raw worker text' }, lang).message
        expect(message).toContain(code)
        expect(message).not.toMatch(/kernelError\.|kernelStep\./)
        expect(message).not.toContain('raw worker text')
      }
    }
  })

  it('routes build errors to the language, kernel or raw presentation', async () => {
    const { presentBuildError } = await import('../src/i18n/kernelErrors')
    const t = (key: string) => translate('ru', key)
    expect(presentBuildError({ code: 'E_FEATURE_INCLUDE', message: 'Line 1' }, 'ru', t)).toMatch(
      /\(E_FEATURE_INCLUDE\) Line 1$/,
    )
    expect(presentBuildError({ code: 'GEOMETRY_PROTOCOL', message: 'Malformed kernel job step' }, 'ru', t)).toBe(
      presentKernelError({ code: 'GEOMETRY_PROTOCOL', message: '' }, 'ru').message,
    )
    expect(presentBuildError({ code: 'SOMETHING_ELSE', message: 'kept' }, 'ru', t)).toBe('kept')
    expect(presentBuildError({ message: 'plain' }, 'ru', t)).toBe('plain')
  })

  it('carries exact-solid worker codes across the protocol and the client', async () => {
    const { isExactSolidResponse } = await import('../src/kernel/solid/exactSolidProtocol')
    const { buildExactSolidsInWorker } = await import('../src/kernel/solid/exactSolidClient')
    const failed = (error: Record<string, unknown>) => ({ kind: 'exact-solid', version: 1, ok: false, error })
    expect(isExactSolidResponse(failed({ name: 'Error', message: 'm', code: 'EXACT_SOLID_NOT_BREP' }))).toBe(true)
    expect(isExactSolidResponse(failed({ name: 'Error', message: 'm' }))).toBe(true)
    expect(isExactSolidResponse(failed({ name: 'Error', message: 'm', code: 'lower case' }))).toBe(false)
    expect(isExactSolidResponse(failed({ name: 'Error', message: 'm', extra: 'x' }))).toBe(false)
    const port = (respond: (listener: EventListener, type: string) => void) => () => {
      const listeners = new Map<string, EventListener>()
      return {
        postMessage() {
          queueMicrotask(() => {
            for (const [type, listener] of [...listeners]) respond(listener, type)
          })
        },
        terminate() {},
        addEventListener(type: string, listener: EventListener) {
          listeners.set(type, listener)
        },
        removeEventListener(type: string) {
          listeners.delete(type)
        },
      }
    }
    const coded = buildExactSolidsInWorker(
      'cube(1);',
      undefined,
      port((listener, type) => {
        if (type === 'message')
          listener({ data: failed({ name: 'Error', message: 'mesh only', code: 'EXACT_SOLID_NOT_BREP' }) } as never)
      }),
    )
    await expect(coded).rejects.toMatchObject({ code: 'EXACT_SOLID_NOT_BREP', message: 'mesh only' })
    const crashed = buildExactSolidsInWorker(
      'cube(1);',
      undefined,
      port((listener, type) => {
        if (type === 'error') listener({} as never)
      }),
    )
    const error = await crashed.catch((caught: unknown) => caught)
    expect(error).toMatchObject({ code: 'CAD_CRASH' })
    expect(presentKernelError(error, 'ru').message).toContain('CAD_CRASH')
  })

  it('tints only the refused faces or the faces next to refused edges', async () => {
    const { invalidSubshapeTriangles, subshapeTriangleHue } =
      await import('../src/features/directModeler/invalidSubshape')
    const topology = {
      faces: [{ triangles: [0, 1] }, { triangles: [2, 3] }, { triangles: [4, 5] }],
      edges: [{ faces: [0, 1] as [number, number] }, { faces: [1, 2] as [number, number] }],
    }
    const selection = { faceIndexes: [2], edgeIndexes: [1] }
    expect(invalidSubshapeTriangles({ input: 'edge' }, topology, selection)).toEqual([2, 3, 4, 5])
    expect(invalidSubshapeTriangles({ input: 'field', field: 'radius' }, topology, selection)).toEqual([2, 3, 4, 5])
    expect(invalidSubshapeTriangles({ input: 'face' }, topology, selection)).toEqual([4, 5])
    expect(invalidSubshapeTriangles({ input: 'field', field: 'distance' }, topology, selection)).toEqual([4, 5])
    expect(invalidSubshapeTriangles({ input: 'body' }, topology, selection)).toEqual([])
    expect(invalidSubshapeTriangles({ input: 'edge' }, topology, { faceIndexes: [-1], edgeIndexes: [-1] })).toEqual([])
    const invalid = new Set([2, 3])
    expect([0, 2, 3, 4].map(i => subshapeTriangleHue(i, 220, invalid, 0))).toEqual([220, 0, 0, 220])
    expect(subshapeTriangleHue(2, 220, undefined, 0)).toBe(220)
  })
})
