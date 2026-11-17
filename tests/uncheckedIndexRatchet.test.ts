import { describe, expect, it } from 'vitest'
import { CLEAN_PATHS, regressions } from '../scripts/check-unchecked-index.mjs'

describe('noUncheckedIndexedAccess ratchet', () => {
  it('reports only errors inside clean paths', () => {
    const output = [
      "src/composables/useModalDialog.ts(137,5): error TS2532: Object is possibly 'undefined'.",
      "src/kernel/geometry/kernel.ts(5,30): error TS2532: Object is possibly 'undefined'.",
      "src/main.ts(53,3): error TS2322: Type 'string | undefined' is not assignable to type 'string'.",
      'src/main.tsx(1,1): error TS2322: unrelated.',
      'Found 3 errors.',
    ].join('\n')
    expect(regressions(output)).toEqual([
      "src/composables/useModalDialog.ts(137,5): error TS2532: Object is possibly 'undefined'.",
      "src/main.ts(53,3): error TS2322: Type 'string | undefined' is not assignable to type 'string'.",
    ])
  })

  it('normalizes Windows separators', () => {
    expect(regressions('src\\dev\\vrEmulator.ts(51,62): error TS2532: x', ['src/dev/'])).toHaveLength(1)
  })

  it('covers the migrated directories', () => {
    expect(CLEAN_PATHS).toEqual(expect.arrayContaining(['src/composables/', 'src/data/', 'src/dev/', 'src/main.ts']))
  })
})
