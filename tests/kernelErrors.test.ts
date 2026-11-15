import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { APP_STRINGS } from '../src/i18n/appStrings'
import {
  DirectInputError,
  KERNEL_ERROR_CODES,
  KERNEL_ERROR_REASONS,
  KERNEL_ERROR_STEPS,
  kernelErrorSpec,
  presentKernelError,
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
