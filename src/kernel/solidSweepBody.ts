import { DirectInputError } from '../i18n/kernelErrors'
import { parseDirectDocument, type DirectDocument, type DirectBody } from '../app/directModeling'
import { directExtrusionTargets, type DirectExtrusionOperation } from '../app/directExtrusion'
import { stringifyMeshJson } from '../exchange/meshJson'
import { reverseNurbsCurve, type NurbsCurve } from './nurbsCurve'
import { buildSmoothSweepToolBrep } from './solidSweepSmoothPath'
import { booleanNurbsBrep, createMiterBrepProfileBody, tessellateNurbsBrep, type NurbsBrep } from './geometry/brep'

/** Sweep result step (C9): keep the open surface, or close the profile into a solid and combine it. */
export type SweepResultMode = 'surface' | DirectExtrusionOperation
export interface SolidSweepBodyOptions {
  profileId: string
  pathId: string
  reversed?: boolean[]
  operation: DirectExtrusionOperation
  targetId: string
  targetIds?: string[]
  id: string
  name: string
  tessellation?: number
  /** Chordal tolerance for sampling a smooth (arc, circle, spline) path, in mm. */
  pathTolerance?: number
}

const lift = (p: readonly number[]): [number, number, number] => [p[0] ?? 0, p[1] ?? 0, p[2] ?? 0]
const sub = (a: readonly number[], b: readonly number[]) => [a[0]! - b[0]!, a[1]! - b[1]!, a[2]! - b[2]!]
const cross = (a: readonly number[], b: readonly number[]): [number, number, number] => [
  a[1]! * b[2]! - a[2]! * b[1]!,
  a[2]! * b[0]! - a[0]! * b[2]!,
  a[0]! * b[1]! - a[1]! * b[0]!,
]

function curveById(document: DirectDocument, id: string, reversed: boolean): NurbsCurve {
  const item = document.curves?.find(c => c.id === id)
  if (!item) throw new DirectInputError('DIRECT_CURVE_MISSING', 'Curve missing. Choose inputs again.', undefined, [id])
  return reversed ? reverseNurbsCurve(item.curve) : item.curve
}

/**
 * Solid sweep tool: the closed profile is carried along a polyline path with mitred stations
 * and capped at both ends. Arc, circle and spline paths go through `buildSmoothSweepToolBrep`.
 * The profile stays where it was drawn and must lie in the plane normal to the path start.
 */
export function buildSweepToolBrep(
  document: DirectDocument,
  options: Pick<SolidSweepBodyOptions, 'profileId' | 'pathId' | 'reversed' | 'pathTolerance'>,
): NurbsBrep {
  if (options.profileId === options.pathId)
    throw new DirectInputError('DIRECT_CURVES_DISTINCT', 'Choose distinct curves.', undefined, [options.profileId])
  const profile = curveById(document, options.profileId, !!options.reversed?.[0])
  const path = curveById(document, options.pathId, !!options.reversed?.[1])
  const ends = [profile.controlPoints[0], profile.controlPoints.at(-1)].map(p => lift(p ?? []))
  if (!profile.periodic && Math.hypot(...sub(ends[0]!, ends[1]!)) > 1e-7)
    throw new DirectInputError('DIRECT_SURFACE_INPUTS', 'A solid sweep needs a closed profile curve.', undefined, [
      options.profileId,
    ])
  // C9: arc, circle and spline paths are sampled to a chordal tolerance and the profile is
  // carried along them with a rotation-minimizing frame.
  if (path.degree !== 1 || path.periodic)
    return buildSmoothSweepToolBrep(profile, path, [options.profileId, options.pathId], options.pathTolerance)
  const points = path.controlPoints
    .map(lift)
    .filter((p, i, all) => i === 0 || Math.hypot(...sub(p, all[i - 1]!)) > 1e-9)
  if (points.length < 2)
    throw new DirectInputError('DIRECT_SURFACE_INPUTS', 'The sweep path needs two distinct points.', undefined, [
      options.pathId,
    ])
  const tangent = sub(points[1]!, points[0]!),
    axis = [0, 1, 2].reduce((m, i) => (Math.abs(tangent[i]!) < Math.abs(tangent[m]!) ? i : m), 0)
  const normal = cross(
    tangent,
    [0, 1, 2].map(i => (i === axis ? 1 : 0)),
  )
  const loop = { ...profile, controlPoints: profile.controlPoints.map(lift) }
  try {
    return createMiterBrepProfileBody([[loop]], points, normal, 4).model
  } catch (error) {
    if (String(error).includes('normal plane'))
      throw new DirectInputError(
        'DIRECT_SURFACE_INPUTS',
        'The profile must lie in the plane normal to the first path segment.',
        undefined,
        [options.profileId, options.pathId],
      )
    throw error
  }
}

function sweepBody(
  document: DirectDocument,
  tool: NurbsBrep,
  options: SolidSweepBodyOptions,
  targetId: string,
): DirectBody | null {
  const target = document.bodies.find(b => b.id === targetId)
  if (options.operation !== 'new' && !target)
    throw new DirectInputError('DIRECT_TARGET_BODY', 'Select a target body.', undefined, [targetId])
  if (options.operation !== 'new' && !target!.brep)
    throw new DirectInputError('DIRECT_TARGET_BODY', 'Sweep Boolean requires an authored B-rep target.', undefined, [
      targetId,
    ])
  const brep = options.operation === 'new' ? tool : booleanNurbsBrep(target!.brep!, tool, options.operation)
  if (!brep.bodies.length) return null
  const mesh = tessellateNurbsBrep(brep, options.tessellation ?? 24)
  return {
    ...(options.operation === 'new' ? { id: options.id, name: options.name } : target!),
    brep,
    mesh: { positions: mesh.positions, indices: mesh.indices },
  }
}

/**
 * One result for preview and commit. Each target is combined independently with the same
 * tool and every target is computed before the document changes, so a refusal on any target
 * leaves all bodies intact. Empty results remove the target.
 */
export function applySolidSweepBody(document: DirectDocument, options: SolidSweepBodyOptions): DirectDocument {
  const targets = directExtrusionTargets(options)
  const tool = buildSweepToolBrep(document, options)
  if (options.operation === 'new') {
    const body = sweepBody(document, tool, options, '')
    return parseDirectDocument(
      stringifyMeshJson({ ...document, bodies: [...document.bodies, ...(body ? [body] : [])] }),
    )
  }
  const results = new Map(targets.map(id => [id, sweepBody(document, tool, options, id)] as const))
  const bodies = document.bodies.flatMap(b => {
    if (!results.has(b.id)) return [b]
    const next = results.get(b.id)
    return next ? [next] : []
  })
  return parseDirectDocument(stringifyMeshJson({ ...document, bodies }))
}
