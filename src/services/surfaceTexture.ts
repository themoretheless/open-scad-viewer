import type {DirectBody} from './directModeling'
import type {Vec3} from './directSketchGeometry'
import {callGeometryRust} from './geometry/kernel'
export type SurfacePattern='ribs'|'grooves'|'knurl'|'fuzzy'|'dimples'|'waves'
export interface SurfaceTextureOptions {pattern:SurfacePattern;pitch:number;height:number;angle:number;seed:number;detail:number;invert:boolean;triangles?:number[];origin:Vec3;u:Vec3;v:Vec3}
export function textureHeight(p:number[],o:SurfaceTextureOptions):number {
 return callGeometryRust('cad_texture_height',{point:p,options:o})
}
/** Native shared-edge refinement and deterministic mesh displacement. */
export function textureSurface(body:DirectBody,o:SurfaceTextureOptions):DirectBody {
 return callGeometryRust('cad_texture_body',{body,options:o})
}
