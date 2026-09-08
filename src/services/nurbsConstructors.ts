/** Surface construction is implemented by the own Rust kernel. */
import { type NurbsCurve } from './nurbsCurve'
import { type NurbsSurface } from './nurbsSurface'
import { callNurbsRust } from './nurbsRustKernel'
export function loftNurbsCurves(curves: NurbsCurve[]): NurbsSurface { return callNurbsRust('loft', { curves }) }
export function extrudeNurbsCurve(curve: NurbsCurve, vector: number[]): NurbsSurface { return callNurbsRust('extrude', { curve, vector }) }
export function revolveNurbsCurve(curve: NurbsCurve, origin: number[], axis: number[], angle: number): NurbsSurface { return callNurbsRust('revolve', { curve, origin, axis, angle }) }
/** Exact translation sweep; the profile frame does not rotate along the path. */
export const sweepNurbsCurve=(profile:NurbsCurve,path:NurbsCurve):NurbsSurface=>callNurbsRust('surface_sweep',{profile,path})
export const loftAlignedNurbsCurves=(curves:NurbsCurve[]):NurbsSurface=>callNurbsRust('loft_aligned',{curves})

import type {GeometryDeformation,GeometryBrush} from './geometryEditing'
/** Nonlinear edits affect control points, not the exact pointwise surface image. */
export const deformNurbsCurve=(curve:NurbsCurve,deformation:GeometryDeformation):NurbsCurve=>callNurbsRust('nurbs_deform_curve',{curve,deformation})
export const deformNurbsSurface=(surface:NurbsSurface,deformation:GeometryDeformation):NurbsSurface=>callNurbsRust('nurbs_deform_surface',{surface,deformation})
export const brushNurbsCurve=(curve:NurbsCurve,brush:GeometryBrush):NurbsCurve=>callNurbsRust('nurbs_brush_curve',{curve,brush})
export const brushNurbsSurface=(surface:NurbsSurface,brush:GeometryBrush):NurbsSurface=>callNurbsRust('nurbs_brush_surface',{surface,brush})
