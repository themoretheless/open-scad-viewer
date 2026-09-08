import {sha256Hex} from './sha256'
import type {SceneEntityId} from './mesh'

// Preserve existing short identities; deep operation paths still fit protocol v5.
export function boundedSceneEntityId(path: string): SceneEntityId {
  const id: SceneEntityId = `entity:${path}`
  return id.length <= 256 ? id : `entity:sha256:${sha256Hex(path)}`
}
