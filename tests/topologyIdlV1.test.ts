import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import {
  applyPatch,
  localValidatorReject,
  schemaReject,
  type Json,
  type Snapshot,
} from './support/referenceTopologyIdlV1'

const repositoryRoot = resolve(import.meta.dirname, '..')
const idlRoot = resolve(repositoryRoot, 'docs/qualification/topology-idl')

describe('G0.6 topology IDL fixtures', () => {
  const box = JSON.parse(
    readFileSync(resolve(idlRoot, 'box-solid-v1.json'), 'utf8'),
  ) as Snapshot
  const mutations = JSON.parse(
    readFileSync(resolve(idlRoot, 'mutations-v1.json'), 'utf8'),
  ) as {
    readonly mutations: readonly {
      readonly id: string
      readonly expected: string
      readonly detail?: string
      readonly patch: {
        readonly op: string
        readonly path: string
        readonly value?: Json
      }
    }[]
  }

  it('accepts the canonical box solid', () => {
    expect(schemaReject(box)).toBeNull()
    expect(localValidatorReject(box)).toBeNull()
    expect(box.vertices).toHaveLength(8)
    expect(box.edges).toHaveLength(12)
    expect(box.faces).toHaveLength(6)
    expect(box.solids[0]?.shellUses[0]?.role).toBe('Outer')
  })

  it('fails each one-invariant mutation for the declared reason class', () => {
    for (const mutation of mutations.mutations) {
      const mutated = applyPatch(box as unknown as Json, mutation.patch) as Snapshot
      if (mutation.expected === 'schema-reject') {
        expect(schemaReject(mutated), mutation.id).not.toBeNull()
      } else if (mutation.expected === 'local-validator-reject') {
        expect(schemaReject(mutated), mutation.id).toBeNull()
        expect(localValidatorReject(mutated), mutation.id).toBe(
          mutation.detail ?? expect.any(String),
        )
      } else {
        throw new Error(`unknown expected ${mutation.expected}`)
      }
    }
  })
})
