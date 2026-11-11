import type { NurbsCurve } from '../nurbsCurve'
import { progressiveMiterPayload } from '../nurbsConstructors'
import type { NurbsScaleLaw, ProgressiveGuidedSurfaceSweepOptions, ProgressiveMiterCapParallelism, ProgressiveMiterCapProjection, ProgressiveMiterIdealCapDomains, ProgressiveMiterOptions, ProgressiveMiterPreview, ProgressiveMiterResult, ProgressiveMultiSweepResult, ProgressiveSweepPreview, ProgressiveSweepStreamOptions } from '../nurbsConstructors'
import type { NurbsSurface } from '../nurbsSurface'
import { callNurbsRust } from '../geometry/nurbs'
import { streamNativeSweep } from '../sweep/transport/streamNative'

export function loftNurbsCurves(curves: NurbsCurve[]): NurbsSurface { return callNurbsRust('loft', { curves }) }

export function extrudeNurbsCurve(curve: NurbsCurve, vector: number[]): NurbsSurface { return callNurbsRust('extrude', { curve, vector }) }

export function revolveNurbsCurve(curve: NurbsCurve, origin: number[], axis: number[], angle: number): NurbsSurface { return callNurbsRust('revolve', { curve, origin, axis, angle }) }

export function inspectProgressiveMiterCapProjection(
 profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveMiterOptions,caps:[NurbsSurface,NurbsSurface],maxCells=10000,maxExactWork=1000000,
):ProgressiveMiterCapProjection {
 return callNurbsRust('brep_sweep_constructor',{operation:'curve_progressive_miter_cap_projection',...progressiveMiterPayload(profiles,points,scale,twist,options),caps,maxCells,maxExactWork})
}

export function inspectProgressiveMiterCapParallelism(
 profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveMiterOptions,caps:[NurbsSurface,NurbsSurface],maxCells=10000,maxExactWork=1000000,
):ProgressiveMiterCapParallelism {
 return callNurbsRust('brep_sweep_constructor',{operation:'curve_progressive_miter_cap_parallelism',...progressiveMiterPayload(profiles,points,scale,twist,options),caps,maxCells,maxExactWork})
}

/** Original material ownership only. Filled-cap error and shell orientation are separate. */
export function inspectProgressiveMiterIdealCapDomains(
 profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveMiterOptions,loopSizes:number[],budgets={tolerance:.001,maxPairs:1000,maxCells:10000,maxExactWork:1000000},
):ProgressiveMiterIdealCapDomains {
 return callNurbsRust('brep_sweep_constructor',{operation:'curve_progressive_miter_cap_domains',...progressiveMiterPayload(profiles,points,scale,twist,options),loopSizes,...budgets})
}

/** Audit retained walls; path-neighbor declarations are derived by Rust. */
export function inspectProgressiveMiterWalls(
 profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveMiterOptions,sections:NurbsCurve[][],
 budgets:Omit<import('../nurbsSweepAudit').SweepWallAuditOptions,'sharedBoundaries'>,
 loopSizes?:number[],
):import('../nurbsSweepAudit').SweepWallAudit {
 return callNurbsRust('brep_sweep_constructor',{operation:'curve_progressive_miter_wall_audit',
  ...progressiveMiterPayload(profiles,points,scale,twist,options),sections,...budgets,...(loopSizes?{loopSizes}:{}),
 })
}

export async function* streamProgressiveMiterNurbsProfiles(profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveMiterOptions,control:ProgressiveSweepStreamOptions={}):AsyncGenerator<ProgressiveMiterPreview,ProgressiveMiterResult,void>{return yield* streamNativeSweep({kind:'raw-miter',profiles,points,scale,twist,options},control)}

/** Yields bounded previews and returns construction patches only after acceptance.
 * Cancellation is cooperative between synchronous kernel calls, not within one call.
 */
export async function* streamProgressiveNurbsProfiles(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,control:ProgressiveSweepStreamOptions={}):AsyncGenerator<ProgressiveSweepPreview,ProgressiveMultiSweepResult,void>{return yield* streamNativeSweep({kind:'raw-profile',profiles,path,scale,twist,options},control)}
