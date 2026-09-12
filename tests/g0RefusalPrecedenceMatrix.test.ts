import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const root = resolve(import.meta.dirname, '..')

describe('G0.3 refusal precedence matrix', () => {
  const matrix = JSON.parse(
    readFileSync(resolve(root, 'docs/qualification/g0-refusal-precedence-matrix-v1.json'), 'utf8'),
  ) as {
    readonly matrixId: string
    readonly stages: readonly string[]
    readonly rows: readonly { readonly id: string; readonly stage: string }[]
  }
  const routing = JSON.parse(
    readFileSync(resolve(root, 'docs/qualification/geometry-routing-contract-v1.json'), 'utf8'),
  ) as { readonly errorPrecedence: readonly string[] }

  it('copies the nine normative errorPrecedence stages', () => {
    expect(matrix.matrixId).toBe('g0-refusal-precedence-matrix-v1')
    expect(matrix.stages).toEqual(routing.errorPrecedence)
    expect(matrix.stages).toHaveLength(9)
  })

  it('indexes routing surface cases and supervisor fixtures', () => {
    expect(matrix.rows.length).toBeGreaterThanOrEqual(18)
    expect(matrix.rows.every((row) => matrix.stages.includes(row.stage))).toBe(true)
    expect(matrix.rows.some((row) => row.id.startsWith('supervisor-'))).toBe(true)
  })
})
