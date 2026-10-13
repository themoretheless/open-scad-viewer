/** Validators for untrusted plain-data protocol messages: no prototypes, getters or extra keys. */

export function record(value: unknown): Record<string, unknown> | null {
  if (value === null || Array.isArray(value) || typeof value !== 'object') return null
  const prototype = Object.getPrototypeOf(value)
  return prototype === Object.prototype || prototype === null
    ? value as Record<string, unknown>
    : null
}

export function ownData(value: object, key: string): unknown {
  const descriptor = Object.getOwnPropertyDescriptor(value, key)
  return descriptor !== undefined && Object.hasOwn(descriptor, 'value')
    ? descriptor.value
    : undefined
}

export function exactDataKeys(value: Record<string, unknown>, keys: readonly string[]): boolean {
  const descriptors = Object.getOwnPropertyDescriptors(value)
  const actual = Reflect.ownKeys(descriptors)
  return actual.length === keys.length
    && actual.every(key => typeof key === 'string' && keys.includes(key))
    && keys.every(key => {
      const descriptor = descriptors[key]
      return descriptor !== undefined
        && descriptor.enumerable
        && Object.hasOwn(descriptor, 'value')
    })
}

export function nonNegativeSafeInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0
}

export function positiveSafeInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) > 0
}
