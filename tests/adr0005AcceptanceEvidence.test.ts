import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const root = resolve(import.meta.dirname, '..')

function readJson<T>(rel: string): T {
  return JSON.parse(readFileSync(resolve(root, rel), 'utf8')) as T
}

describe('ADR 0005 acceptance evidence', () => {
  const evidence = readJson<{
    readonly gates: readonly {
      readonly id: string
      readonly status: string
      readonly evidence: readonly { readonly path: string }[]
    }[]
  }>('docs/qualification/adr-0005-acceptance-evidence-v1.json')

  it('closes the three previously remaining FAIL gates', () => {
    expect(evidence.gates.map((g) => g.id)).toEqual([
      'shadow-adapter-source-to-event',
      'worker-identity-256-257',
      'mcp-hard-cancel-join',
    ])
    for (const gate of evidence.gates) {
      expect(gate.status, gate.id).toBe('closed')
      for (const item of gate.evidence) {
        expect(readFileSync(resolve(root, item.path)).byteLength, item.path).toBeGreaterThan(16)
      }
    }
  })

  it('freezes Worker 256/257 boundary rows', () => {
    const boundary = readJson<{
      readonly identityUtf16Limit: number
      readonly rows: readonly { readonly utf16CodeUnits: number; readonly expected: string }[]
    }>('docs/qualification/adr-0005-worker-identity-boundary-v1.json')
    expect(boundary.identityUtf16Limit).toBe(256)
    expect(boundary.rows.filter((r) => r.utf16CodeUnits === 256)).toHaveLength(3)
    expect(boundary.rows.filter((r) => r.utf16CodeUnits === 257)).toHaveLength(3)
    expect(
      boundary.rows
        .filter((r) => r.utf16CodeUnits === 257)
        .every((r) => r.expected === 'bounded-failed-terminal'),
    ).toBe(true)
  })

  it('requires cancel traces to forbid late success', () => {
    const traces = readJson<{
      readonly traces: readonly {
        readonly id: string
        readonly lateSuccessForbidden?: boolean
        readonly terminalCount?: number
      }[]
    }>('docs/qualification/adr-0005-shadow-adapter-traces-v1.json')
    const cancel = traces.traces.find((t) => t.id === 'cancel-before-success')
    expect(cancel?.lateSuccessForbidden).toBe(true)
    expect(cancel?.terminalCount).toBe(1)
  })
})
