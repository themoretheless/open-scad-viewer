import { describe, expect, it } from 'vitest'
import { toWorkerErrorInfo, validWorkerErrorField, validWorkerErrorTarget } from '../src/core/workerHandlerRuntime'
import { DirectInputError } from '../src/i18n/kernelErrors'

describe('worker error field and target keys', () => {
  it('serializes the field and target ids of a coded refusal', () => {
    const info = toWorkerErrorInfo(new DirectInputError('DIRECT_PATTERN_GRID', 'Grid', 'patternCount', ['body-1']))
    expect(info).toEqual({
      name: 'DirectInputError',
      message: 'Grid',
      code: 'DIRECT_PATTERN_GRID',
      field: 'patternCount',
      target: ['body-1'],
    })
  })

  it('omits absent or invalid keys instead of sending undefined', () => {
    const plain = toWorkerErrorInfo(new DirectInputError('DIRECT_TARGET_BODY', 'x'))
    expect(Object.keys(plain).sort()).toEqual(['code', 'message', 'name'])
    const bad = Object.assign(new Error('x'), { field: 'f'.repeat(65), targetIds: [1] })
    expect(Object.keys(toWorkerErrorInfo(bad)).sort()).toEqual(['message', 'name'])
    expect(Object.keys(toWorkerErrorInfo('text')).sort()).toEqual(['message', 'name'])
  })

  it('validates the keys with exact bounds', () => {
    expect(validWorkerErrorField(undefined)).toBe(true)
    expect(validWorkerErrorField('radius')).toBe(true)
    expect(validWorkerErrorField('')).toBe(false)
    expect(validWorkerErrorField('x'.repeat(64))).toBe(true)
    expect(validWorkerErrorField('x'.repeat(65))).toBe(false)
    expect(validWorkerErrorTarget(undefined)).toBe(true)
    expect(validWorkerErrorTarget(['a'])).toBe(true)
    expect(validWorkerErrorTarget([])).toBe(false)
    expect(validWorkerErrorTarget(['a'.repeat(201)])).toBe(false)
    expect(validWorkerErrorTarget(Array.from({ length: 64 }, (_, i) => `b${i}`))).toBe(true)
    expect(validWorkerErrorTarget(Array.from({ length: 65 }, (_, i) => `b${i}`))).toBe(false)
  })
})
