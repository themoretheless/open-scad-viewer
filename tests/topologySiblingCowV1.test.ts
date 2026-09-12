import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import { schemaReject, type Snapshot } from './support/referenceTopologyIdlV1'

const repositoryRoot = resolve(import.meta.dirname, '..')
const idlRoot = resolve(repositoryRoot, 'docs/qualification/topology-idl')

describe('G0.6 sibling COW / stale-key fixtures', () => {
  const box = JSON.parse(
    readFileSync(resolve(idlRoot, 'box-solid-v1.json'), 'utf8'),
  ) as Snapshot
  const sibling = JSON.parse(
    readFileSync(resolve(idlRoot, 'sibling-cow-stale-key-v1.json'), 'utf8'),
  ) as {
    readonly cases: readonly {
      readonly id: string
      readonly kind: string
      readonly parentSolidTopoId?: string
      readonly siblingSolidTopoId?: string
      readonly staleSolidTopoId?: string
      readonly liveSolidTopoId?: string
      readonly expect: string
    }[]
  }

  it('keeps parent and sibling solid TopoIds distinct', () => {
    const row = sibling.cases.find((c) => c.id === 'sibling-distinct-topo-ids')
    expect(row?.parentSolidTopoId).not.toBe(row?.siblingSolidTopoId)
    expect(row?.expect).toBe('distinct-solid-topo-ids')
  })

  it('rejects stale TopoId against the live sibling identity', () => {
    const row = sibling.cases.find((c) => c.id === 'stale-key-after-commit')
    expect(row?.staleSolidTopoId).not.toBe(row?.liveSolidTopoId)
    expect(row?.staleSolidTopoId).not.toBe(box.topoIds.solid)
    expect(row?.expect).toBe('stale-topo-id-reject')
  })

  it('still forbids arena keys on the public box IDL', () => {
    const forged = {
      ...box,
      vertices: box.vertices.map((v, i) =>
        i === 0 ? { ...v, arenaKey: { slot: 0, generation: 1 } } : v,
      ),
    } as Snapshot
    expect(schemaReject(forged)).toBe('arena-key-forbidden')
  })
})
