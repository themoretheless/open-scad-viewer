import {callGeometryRust} from './geometry/kernel'
import type {NurbsSurface} from './nurbsSurface'
import type {NurbsSurfaceBoundary} from './solidNurbs'
export interface SurfaceBoundaryOptions {boundaryA:NurbsSurfaceBoundary;boundaryB:NurbsSurfaceBoundary;reverse:boolean;samples:number;toleranceMm:number;angleToleranceDeg:number}
export interface SurfaceBoundaryReport {
 samplingOnly:true;maxGapMm:number;maxTangentPlaneAngleDeg:number|null;undefinedNormals:number;worstGapSample:number;sampledWithinTolerance:boolean
 samples:{t:number;a:number[];b:number[];gapMm:number;tangentPlaneAngleDeg:number|null}[]
}
export const measureSurfaceBoundaries=(a:NurbsSurface,b:NurbsSurface,options:SurfaceBoundaryOptions):SurfaceBoundaryReport=>callGeometryRust('cad_surface_boundary_measure',{a,b,...options})
