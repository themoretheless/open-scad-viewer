import { clampGestureDistance } from '../cameraGestures'
import { verifiedEqualPairs } from '../webgpuRenderer'

export function sampledLanesMatch(left: Float32Array | Uint32Array, right: Float32Array | Uint32Array) {
  const length = left.length
  if (left[0] !== right[0] || left[length - 1] !== right[length - 1]) return false
  const lanes = Math.min(62, length)
  const step = length / lanes
  for (let lane = 0; lane < lanes; lane++) {
    const index = Math.floor(lane * step)
    if (left[index] !== right[index]) return false
  }
  return true
}

export function sameTypedArray(left: Float32Array | Uint32Array, right: Float32Array | Uint32Array) {
  if (left === right) return true
  if (left.constructor !== right.constructor || left.length !== right.length) return false
  if (left.length === 0) return true
  if (!sampledLanesMatch(left, right)) return false
  if (verifiedEqualPairs.get(left)?.has(right)) return true
  for (let index = 0; index < left.length; index++) if (left[index] !== right[index]) return false
  let verified = verifiedEqualPairs.get(left)
  if (!verified) verifiedEqualPairs.set(left, verified = new WeakSet())
  verified.add(right)
  return true
}

/** Positions-only (stride 3) copy of an interleaved position+normal vertex array. */
export function extractPositions(vertices: Float32Array): Float32Array {
  const positions = new Float32Array(vertices.length / 2)
  for (let source = 0, target = 0; source < vertices.length; source += 6, target += 3) {
    positions[target] = vertices[source]
    positions[target + 1] = vertices[source + 1]
    positions[target + 2] = vertices[source + 2]
  }
  return positions
}

/** Local positions a morph displays at `now`, for chaining a new morph mid-flight. */
export function blendMorphPositions(morph: { from: Float32Array; target: Float32Array; started: number }, now: number): Float32Array {
  const t = Math.min(1, Math.max(0, (now - morph.started) / 180))
  const eased = t * t * (3 - 2 * t)
  const target = morph.target
  const out = new Float32Array(morph.from.length)
  for (let index = 0, vertex = 0; index < out.length; index += 3, vertex += 6) {
    out[index] = morph.from[index] + (target[vertex] - morph.from[index]) * eased
    out[index + 1] = morph.from[index + 1] + (target[vertex + 1] - morph.from[index + 1]) * eased
    out[index + 2] = morph.from[index + 2] + (target[vertex + 2] - morph.from[index + 2]) * eased
  }
  return out
}

export function clampDistanceValue(distance: number) {
  return clampGestureDistance(distance)
}
