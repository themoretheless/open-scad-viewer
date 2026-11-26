import { callNurbsRust } from './geometry/nurbs'
import type { NurbsCurve } from './nurbsCurve'

export function editNurbsCurveWeights(curve: NurbsCurve, edits: [number, number][]): NurbsCurve {
  return callNurbsRust('curve_edit_weights', { curve, edits: edits.map(([index, weight]) => ({ index, weight })) })
}

export function relocateNurbsCurveSeam(curve: NurbsCurve, parameter: number, tolerance: number, maxCells: number): NurbsCurve {
  return callNurbsRust('curve_relocate_seam', { curve, parameter, tolerance, maxCells })
}

export function curveExactInsertKnot(curve: NurbsCurve, parameter: number, count: number, max_work: number): NurbsCurve {
  return callNurbsRust('curve_exact_insert_knot', { curve, parameter, count, max_work })
}

export function curveExactRemoveKnot(curve: NurbsCurve, parameter: number, max_work: number): NurbsCurve {
  return callNurbsRust('curve_exact_remove_knot', { curve, parameter, max_work })
}

export function curveExactReduceDegree(curve: NurbsCurve, max_work: number): NurbsCurve {
  return callNurbsRust('curve_exact_reduce_degree', { curve, max_work })
}

export function curveNormalizeWeights(curve: NurbsCurve): NurbsCurve {
  return callNurbsRust('curve_normalize_weights', { curve })
}

export function curveCancelFactor(curve: NurbsCurve, max_work: number): NurbsCurve {
  return callNurbsRust('curve_cancel_factor', { curve, max_work })
}

export function curveExactSplit(curve: NurbsCurve, parameter: number, part: number, max_work: number): NurbsCurve {
  return callNurbsRust('curve_exact_split', { curve, parameter, part, max_work })
}

export function curveExactRestrict(curve: NurbsCurve, domain: [number, number], max_work: number): NurbsCurve {
  return callNurbsRust('curve_exact_restrict', { curve, domain, max_work })
}

export function curveExactPartition(curve: NurbsCurve, cuts: number[], part: number, max_work: number): NurbsCurve {
  return callNurbsRust('curve_exact_partition', { curve, cuts, part, max_work })
}

export function curveExactElevate(curve: NurbsCurve, degree: number, max_work: number): NurbsCurve {
  return callNurbsRust('curve_exact_elevate', { curve, degree, max_work })
}
