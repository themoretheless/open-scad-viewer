import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import { sceneReject, type SceneV2 } from './support/referenceGeometrySceneV2'

const root = resolve(import.meta.dirname, '..')
const idl = resolve(root, 'docs/qualification/geometry-scene-v2')

describe('G0.8 GeometrySceneV2 IDL', () => {
  const fixtures = JSON.parse(
    readFileSync(resolve(idl, 'wire-fixtures-v1.json'), 'utf8'),
  ) as {
    readonly fixtures: readonly {
      readonly id: string
      readonly kind: string
      readonly expect?: string
      readonly scene: SceneV2
    }[]
  }
  const matrix = JSON.parse(
    readFileSync(resolve(idl, 'v5-migration-matrix-v1.json'), 'utf8'),
  ) as {
    readonly rows: readonly { readonly id: string; readonly expect?: string }[]
  }

  it('accepts positive wire fixtures and rejects declared negatives', () => {
    for (const fixture of fixtures.fixtures) {
      const rejected = sceneReject(fixture.scene)
      if (fixture.kind === 'positive') {
        expect(rejected, fixture.id).toBeNull()
      } else {
        expect(rejected, fixture.id).toBe(fixture.expect ?? expect.any(String))
      }
    }
  })

  it('keeps a fail-closed silent-widening migration row', () => {
    expect(matrix.rows.some((row) => row.id === 'reject-silent-v1-widening')).toBe(true)
    expect(matrix.rows.some((row) => row.id === 'fullEquivalent-check')).toBe(true)
  })
})
