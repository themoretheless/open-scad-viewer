import {callGeometryRust} from './kernel'
import {normalizePolygonMesh} from './polygon'
import type {PolygonBuild} from './polygon'
export interface SubdivisionCage {vertices:number[][];faces:number[][]}
export interface SubdivisionRefined {cage:SubdivisionCage;faceIds:number[]}
export const refineSubdivision=(cage:SubdivisionCage,levels=2):SubdivisionRefined=>callGeometryRust('subdivision_refine',{cage,levels})
export const tessellateSubdivision=(cage:SubdivisionCage,levels=2):PolygonBuild & {faceIds:number[]}=>normalizePolygonMesh(callGeometryRust('subdivision_tessellate',{cage,levels}))

import {validateSculptBrush,type GeometryDeformation,type GeometryBrush,type SculptBrush} from '../geometryEditing'
export const sculptSubdivision=(cage:SubdivisionCage,brush:SculptBrush):SubdivisionCage=>{validateSculptBrush(brush);return callGeometryRust('subdivision_sculpt',{cage,brush})}
export const deformSubdivision=(cage:SubdivisionCage,deformation:GeometryDeformation):SubdivisionCage=>callGeometryRust('subdivision_deform',{cage,deformation})
export const brushSubdivision=(cage:SubdivisionCage,brush:GeometryBrush):SubdivisionCage=>callGeometryRust('subdivision_brush',{cage,brush})
export const extrudeSubdivision=(profile:number[][],vector:number[]):SubdivisionCage=>callGeometryRust('subdivision_extrude',{profile,vector})
export const loftSubdivision=(sections:number[][][],caps=true):SubdivisionCage=>callGeometryRust('subdivision_loft',{sections,caps})
export const sweepSubdivision=(profile:number[][],path:number[][],up=[1,0,0],caps=true):SubdivisionCage=>callGeometryRust('subdivision_sweep',{profile,path,up,caps})
export const revolveSubdivision=(profile:number[][],segments=32):SubdivisionCage=>callGeometryRust('subdivision_revolve',{profile,segments})
