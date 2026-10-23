import { callGeometryRust } from './geometry/kernel'
/** A vertex starts with xyz; remaining linear attributes are interpolated at cuts. */
export type TransparentVertex = readonly number[]
export type TransparentTriangle = readonly [TransparentVertex, TransparentVertex, TransparentVertex]

/** Native clipper interpolates every linear attribute and returns independent arrays. */
export function splitTransparentTriangle(triangle: TransparentTriangle, plane: readonly [number,number,number,number], tolerance=0): {front:number[][][];back:number[][][];coplanar:number[][][]} {
  if(!Number.isFinite(tolerance)||tolerance<0)throw Error('Invalid transparent triangle split')
  return callGeometryRust('transparent_triangle_split',{triangle,plane,tolerance})
}
