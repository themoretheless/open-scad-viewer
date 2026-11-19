import {
  rebuildNurbsCurveCertified, rebuildNurbsSurfaceCertified,
  reduceNurbsCurveDegreeCertified, reduceNurbsSurfaceDegreeCertified,
  removeNurbsCurveKnotCertified, removeNurbsSurfaceKnotCertified,
  type CertifiedNurbsCurveEdit, type CertifiedNurbsSurfaceEdit,
} from '../../kernel/nurbsFoundation'
import type { NurbsCurve } from '../../kernel/nurbsCurve'
import type { NurbsSurface } from '../../kernel/nurbsSurface'

type CurveEdit =
  | { op: 'curve_remove_knot'; parameter: number; max_deviation: number }
  | { op: 'curve_reduce_degree'; degree: number; max_deviation: number }
  | { op: 'curve_rebuild'; degree: number; control_count: number; max_deviation: number }

type SurfaceEdit = (
  | { op: 'surface_remove_knot'; parameter: number }
  | { op: 'surface_reduce_degree'; degree: number }
  | { op: 'surface_rebuild'; degree: number; control_count: number }
) & { direction: 'u' | 'v'; max_deviation: number }

function accepted<T extends CertifiedNurbsCurveEdit | CertifiedNurbsSurfaceEdit>(edit: T): T {
  const { certificate } = edit
  if (certificate.accepted !== true) {
    throw new Error('NURBS edit exceeds its deviation budget: ' + certificate.hausdorffErrorUpper + ' mm')
  }
  return edit
}

export function approximateCurveEdit(curve: NurbsCurve, edit: CurveEdit): NurbsCurve {
  switch (edit.op) {
    case 'curve_remove_knot':
      return accepted(removeNurbsCurveKnotCertified(curve, edit.parameter, edit.max_deviation)).curve
    case 'curve_reduce_degree':
      return accepted(reduceNurbsCurveDegreeCertified(curve, edit.degree, edit.max_deviation)).curve
    case 'curve_rebuild':
      return accepted(rebuildNurbsCurveCertified(curve, edit.degree, edit.control_count, edit.max_deviation)).curve
  }
}

export function approximateSurfaceEdit(surface: NurbsSurface, edit: SurfaceEdit): NurbsSurface {
  const { direction, max_deviation } = edit
  switch (edit.op) {
    case 'surface_remove_knot':
      return accepted(removeNurbsSurfaceKnotCertified(surface, direction, edit.parameter, max_deviation)).surface
    case 'surface_reduce_degree':
      return accepted(reduceNurbsSurfaceDegreeCertified(surface, direction, edit.degree, max_deviation)).surface
    case 'surface_rebuild':
      return accepted(rebuildNurbsSurfaceCertified(surface, direction, edit.degree, edit.control_count, max_deviation)).surface
  }
}
