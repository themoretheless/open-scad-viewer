import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const root = resolve(import.meta.dirname, '..')
const protocolRoot = resolve(root, 'docs/qualification/protocol-v6')

describe('G0.8 protocol v6 contract pack', () => {
  it('freezes current Worker v6 success fixture shape', () => {
    const fixture = JSON.parse(
      readFileSync(resolve(protocolRoot, 'current-worker-protocol-v6.fixture.json'), 'utf8'),
    ) as {
      protocolVersion: number
      status: string
      reduced: boolean
      meshes: unknown[]
    }
    expect(fixture.protocolVersion).toBe(6)
    expect(fixture.status).toBe('succeeded')
    expect(fixture.reduced).toBe(false)
    expect(Array.isArray(fixture.meshes)).toBe(true)
  })

  it('rejects GeometrySceneV2 fixtures that break fullEquivalent=!reduced', () => {
    const scene = JSON.parse(
      readFileSync(
        resolve(protocolRoot, 'geometry-scene-v2.negative-inconsistent-equivalence.json'),
        'utf8',
      ),
    ) as {
      fullEquivalent: boolean
      reduced: boolean
    }
    expect(scene.fullEquivalent === !scene.reduced).toBe(false)
  })

  it('lists every current message in the migration matrix', () => {
    const matrix = JSON.parse(
      readFileSync(resolve(protocolRoot, 'migration-matrix-v1.json'), 'utf8'),
    ) as { rows: { message: string }[]; newSceneV2Fields: string[] }
    const messages = matrix.rows.map((row) => row.message)
    for (const required of [
      'build request',
      'cancel request',
      'accepted',
      'started',
      'progress',
      'succeeded',
      'failed',
      'cancelled',
      'stale',
    ]) {
      expect(messages).toContain(required)
    }
    expect(matrix.newSceneV2Fields).toContain('topologySnapshotId')
    expect(matrix.newSceneV2Fields).toContain('fullEquivalent')
  })
})

describe('G0.10 comparator artifact', () => {
  it('owns 18 mutation rows outside the G1 plan document', () => {
    const comparator = JSON.parse(
      readFileSync(
        resolve(root, 'docs/qualification/legacy-direct-differential-comparator-v1.json'),
        'utf8',
      ),
    ) as { mutations: unknown[]; comparatorId: string }
    expect(comparator.comparatorId).toBe('legacy-direct-differential-v1')
    expect(comparator.mutations).toHaveLength(18)
  })
})
