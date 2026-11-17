import type { DirectBody, DirectDocument } from '../app/directModeling'
import { DirectInputError } from '../i18n/kernelErrors'
import type { SolidInstanceBatchCache } from './solidInstanceBatchCache'
import { resolveSolidInstances } from './solidInstances'

type Vec3 = [number, number, number]

/**
 * Linked-instance arrays. `step` repeats at a fixed spacing, `extent` spreads
 * `count` placements over a total span, `path` spaces them by arc length along
 * a polyline (for example a sampled curve), optionally turning each copy with
 * the path tangent. Placements exclude the source itself.
 */
export type SolidInstanceArraySpec =
  | { kind: 'step'; direction: Vec3; step: number; count: number }
  | { kind: 'extent'; direction: Vec3; extent: number; count: number }
  | { kind: 'path'; points: Vec3[]; count: number; align?: boolean }

export const MAX_LINKED_INSTANCES = 1000
const MAX_ARRAY_COUNT = 256

const finite = (values: readonly number[]) => values.every(v => Number.isFinite(v) && Math.abs(v) <= 1e6)
const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
const len = (a: Vec3) => Math.hypot(a[0], a[1], a[2])
const unit = (a: Vec3): Vec3 => {
  const l = len(a)
  return [a[0] / l, a[1] / l, a[2] / l]
}
const cross = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
]
const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2]

const translation = (t: Vec3): number[][] => [
  [1, 0, 0, t[0]],
  [0, 1, 0, t[1]],
  [0, 0, 1, t[2]],
  [0, 0, 0, 1],
]

/** Rotation taking unit `from` onto unit `to` about `pivot`, then translated by `offset`. */
function alignedPlacement(from: Vec3, to: Vec3, pivot: Vec3, offset: Vec3): number[][] {
  const c = dot(from, to)
  let axis = cross(from, to)
  let s = len(axis)
  if (s < 1e-12) {
    if (c > 0) return translation(offset)
    // Opposite tangents: half turn about any axis perpendicular to `from`.
    axis = unit(cross(from, Math.abs(from[0]) < 0.9 ? [1, 0, 0] : [0, 1, 0]))
    s = 0
  } else axis = unit(axis)
  const [x, y, z] = axis,
    k = 1 - c
  const r = [
    [c + x * x * k, x * y * k - z * s, x * z * k + y * s],
    [y * x * k + z * s, c + y * y * k, y * z * k - x * s],
    [z * x * k - y * s, z * y * k + x * s, c + z * z * k],
  ]
  const target: Vec3 = [pivot[0] + offset[0], pivot[1] + offset[1], pivot[2] + offset[2]]
  return [...r.map((row, i) => [...row, target[i] - dot(row as Vec3, pivot)]), [0, 0, 0, 1]]
}

function refuse(message: string): never {
  throw new DirectInputError('DIRECT_PATTERN_ARRAY', message, 'patternCount', [])
}

/** Placement matrices for the copies (the source keeps its own placement). */
export function solidInstanceArrayPlacements(spec: SolidInstanceArraySpec): number[][][] {
  if (!Number.isSafeInteger(spec.count) || spec.count < 2 || spec.count > MAX_ARRAY_COUNT)
    refuse(`Array needs 2–${MAX_ARRAY_COUNT} placements.`)
  if (spec.kind === 'step' || spec.kind === 'extent') {
    const length = spec.kind === 'step' ? spec.step : spec.extent
    if (!finite([...spec.direction, length]) || len(spec.direction) < 1e-12 || length === 0)
      refuse('Array needs a finite non-zero direction and spacing.')
    const d = unit(spec.direction),
      step = spec.kind === 'step' ? spec.step : spec.extent / (spec.count - 1)
    return Array.from({ length: spec.count - 1 }, (_, i) =>
      translation([d[0] * step * (i + 1), d[1] * step * (i + 1), d[2] * step * (i + 1)]),
    )
  }
  const points = spec.points
  if (points.length < 2 || points.length > 10_000 || !points.every(p => p.length === 3 && finite(p)))
    refuse('Path array needs 2–10000 finite points.')
  const cumulative = [0]
  for (let i = 1; i < points.length; i++) cumulative.push(cumulative[i - 1] + len(sub(points[i], points[i - 1])))
  const total = cumulative[cumulative.length - 1]
  if (!(total > 1e-9)) refuse('Path array needs a path of non-zero length.')
  const segmentAt = (distance: number) => {
    let i = 1
    while (i < points.length - 1 && cumulative[i] < distance) i++
    // Skip zero-length segments so the tangent stays defined.
    while (i > 1 && cumulative[i] === cumulative[i - 1]) i--
    return i
  }
  const tangentAt = (i: number): Vec3 => {
    for (let j = i; j < points.length; j++) {
      const d = sub(points[j], points[j - 1])
      if (len(d) > 1e-12) return unit(d)
    }
    return unit(sub(points[points.length - 1], points[0]))
  }
  const start = points[0],
    startTangent = tangentAt(1)
  return Array.from({ length: spec.count - 1 }, (_, n) => {
    const distance = (total * (n + 1)) / (spec.count - 1),
      i = segmentAt(distance)
    const a = points[i - 1],
      b = points[i],
      span = cumulative[i] - cumulative[i - 1],
      t = span > 0 ? (distance - cumulative[i - 1]) / span : 1
    const at: Vec3 = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
    const offset = sub(at, start)
    return spec.align ? alignedPlacement(startTangent, tangentAt(i), start, offset) : translation(offset)
  })
}

/**
 * Adds one linked instance of `sourceId` per placement in a single document
 * copy and one resolve. Ids are `${idPrefix}${n}` starting at 1.
 */
export function createSolidInstanceArray(
  document: DirectDocument,
  sourceId: string,
  idPrefix: string,
  placements: readonly number[][][],
  cache?: SolidInstanceBatchCache,
): DirectDocument {
  const source = document.bodies.find(body => body.id === sourceId)
  if (!source || source.instance)
    throw new DirectInputError(
      'DIRECT_PATTERN_SOURCE',
      'Select an independent source body.',
      undefined,
      source ? [source.id] : [],
    )
  const taken = new Set(
    [...document.bodies, ...document.sketches, ...(document.curves ?? []), ...(document.surfaces ?? [])].map(o => o.id),
  )
  const ids = placements.map((_, i) => `${idPrefix}${i + 1}`)
  if (!idPrefix || ids.some(id => id.length > 200 || taken.has(id))) refuse('Array instances need unique object IDs.')
  if (document.bodies.filter(b => b.instance).length + placements.length > MAX_LINKED_INSTANCES)
    refuse(`Arrays exceed ${MAX_LINKED_INSTANCES} linked instances.`)
  const next = structuredClone(document)
  const { livePattern: _pattern, ...template } = next.bodies.find(body => body.id === sourceId)!
  const name = source.name.slice(0, 80)
  next.bodies.push(
    ...placements.map((matrix, i): DirectBody => ({
      ...structuredClone(template),
      id: ids[i],
      name: `${name} · ${i + 2}`,
      instance: { sourceId, matrix: structuredClone(matrix) },
    })),
  )
  return resolveSolidInstances(next, cache)
}
