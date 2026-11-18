import { bodyPoints, type DirectBody, type Point2 } from '../app/directModeling'
import { facePlane, selectedBrepSupport, solidTopology } from '../app/directSolidTools'
import { projectBrepFaceToPlane, projectPolygonFaceToPlane, type ProjectedSketch } from '../app/sketchProjection'
import type { SketchPlane, Vec3 } from '../app/directSketchGeometry'
import { DirectInputError } from '../i18n/kernelErrors'
import { polygonBoundaryLoops } from './geometry/polygon'
/** Prepare the supporting plane and every boundary before publishing a workplane. */
export function prepareSolidFaceSketch(body: DirectBody, index: number) {
  if (!Number.isSafeInteger(index) || index < 0) throw Error('Select a planar face.')
  const face = solidTopology(body.mesh).faces[index]
  if (!face) throw Error('Selected face is unavailable. Select the face again.')
  const plane = facePlane(body, face),
    points = bodyPoints(body)
  const outline = polygonBoundaryLoops({
    positions: body.mesh.positions,
    indices: Uint32Array.from(face.triangles.flatMap(t => Array.from(body.mesh.indices.slice(t * 3, t * 3 + 3)))),
  }).map(loop =>
    loop.map(i => {
      const q = points[i]!.map((v, k) => v - plane.origin[k]!)
      return [q.reduce((s, v, k) => s + v * plane.u[k]!, 0), q.reduce((s, v, k) => s + v * plane.v[k]!, 0)] as Point2
    }),
  )
  if (!outline.length || outline.some(loop => loop.length < 3))
    throw Error('Selected face has no closed boundary. Select another planar face.')
  return { plane, outline, normal: face.normal }
}
/**
 * Exact projection of a body face's edges onto `target`: a body with a retained B-rep projects the supporting B-rep
 * face (lines, and circular arcs from a parallel face); a polyhedral body projects the mesh face boundary loops,
 * whose edges are straight. Refusals use the sketch projection codes.
 */
export function projectSolidFaceToPlane(body: DirectBody, index: number, target: SketchPlane): ProjectedSketch[] {
  const face = Number.isSafeInteger(index) && index >= 0 ? solidTopology(body.mesh).faces[index] : undefined
  if (!face) throw new DirectInputError('DIRECT_SKETCH_PROJECTION_INPUT', 'Select a face to project.')
  if (body.brep) return projectBrepFaceToPlane(body.brep, selectedBrepSupport(body, face.triangles), target)
  const points = bodyPoints(body)
  const loops = polygonBoundaryLoops({
    positions: body.mesh.positions,
    indices: Uint32Array.from(face.triangles.flatMap(t => Array.from(body.mesh.indices.slice(t * 3, t * 3 + 3)))),
  }).map(loop => loop.map(i => [...points[i]!] as Vec3))
  return projectPolygonFaceToPlane(loops, target)
}
