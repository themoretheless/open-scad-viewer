/**
 * Independent G0.8 GeometrySceneV2 checker.
 * Imports no Worker protocol codec or production scene modules.
 */

export type Json =
  | null
  | boolean
  | number
  | string
  | Json[]
  | { readonly [key: string]: Json }

const HEX64 = /^[0-9a-f]{64}$/
const HEX32 = /^[0-9a-f]{32}$/

export type SceneV2 = {
  readonly schema: string
  readonly schemaVersion: number
  readonly sceneVersion: number
  readonly workerEpoch: number
  readonly kernelKey: string
  readonly kernelFingerprint: string
  readonly capabilityManifestVersion: string
  readonly representation: string
  readonly fullEquivalent: boolean
  readonly reduced: boolean
  readonly topologySnapshotId?: string
  readonly packetTopoTokens?: readonly {
    readonly token: number
    readonly topoId: string
  }[]
  readonly occurrences: readonly {
    readonly occurrenceId: string
    readonly meshAssetId: string
    readonly transform: readonly number[]
    readonly color: readonly number[]
  }[]
  readonly assets: readonly {
    readonly meshAssetId: string
    readonly positions: readonly number[]
    readonly indices: readonly number[]
  }[]
  readonly diagnostics: readonly unknown[]
}

export function sceneReject(scene: SceneV2): string | null {
  if (scene.schema !== 'open-scad-viewer/geometry-scene-v2') return 'schema'
  if (scene.schemaVersion !== 1) return 'schemaVersion'
  if (scene.sceneVersion !== 2) return 'sceneVersion'
  if (!Number.isInteger(scene.workerEpoch) || scene.workerEpoch < 0) return 'workerEpoch'
  if (!scene.kernelKey || scene.kernelKey.length > 128) return 'kernelKey'
  if (!HEX64.test(scene.kernelFingerprint)) return 'kernelFingerprint'
  if (!scene.capabilityManifestVersion) return 'capabilityManifestVersion'
  if (!['MeshSolid', 'AnalyticBrep', 'RationalBrep'].includes(scene.representation)) {
    return 'representation'
  }
  if (scene.fullEquivalent !== !scene.reduced) return 'fullEquivalent-reduced-mismatch'
  if (scene.topologySnapshotId !== undefined && !HEX64.test(scene.topologySnapshotId)) {
    return 'topologySnapshotId'
  }
  for (const token of scene.packetTopoTokens ?? []) {
    if (!Number.isInteger(token.token) || token.token < 0) return 'packet-token'
    if (!HEX32.test(token.topoId)) return 'packet-topo-id'
  }
  const assetIds = new Set(scene.assets.map((a) => a.meshAssetId))
  for (const occ of scene.occurrences) {
    if (!assetIds.has(occ.meshAssetId)) return 'occurrence-asset-missing'
    if (occ.transform.length !== 16) return 'transform'
    if (occ.color.length !== 4) return 'color'
  }
  return null
}

export function migrationReject(
  row: { readonly id: string; readonly expect?: string },
  scene?: SceneV2,
): string | null {
  if (row.expect === 'fail-closed') return null
  if (scene) return sceneReject(scene)
  return null
}
