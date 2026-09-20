import {callGeometryRust} from './geometry/kernel'
import type {DirectBody} from './directModeling'
export type LighteningPattern='bone'|'spatial'|'bcc'|'octet'|'isogrid'|'grid'|'triangles'|'honeycomb'|'web'
export function isSpatialPattern(pattern:LighteningPattern|undefined):boolean{
 return pattern==='bone'||pattern==='spatial'||pattern==='bcc'||pattern==='octet'
}
export interface LighteningOptions {pattern:LighteningPattern;axis:'x'|'y'|'z';cell:number;rib:number;rim:number;bottom:number;top:number;seed:number;jitter:number;lineWidth:number;perimeters:number;skin?:number;step?:number;openTop?:boolean;diagonals?:boolean;wallDepth?:number;keepCore?:boolean}
type P=[number,number]
export function lighteningCells(min:P,max:P,o:LighteningOptions):P[][]{
 return callGeometryRust('cad_lightening_cells',{min,max,pattern:o.pattern,cell:o.cell,rib:o.rib,seed:o.seed,jitter:o.jitter})
}
export function latticeComponents(b:DirectBody,positiveOnly=false):number {
 return callGeometryRust('cad_lattice_components',{mesh:b.mesh,positiveOnly})
}
type SpatialGraphResult={nodes:number[][];edges:number[][]}
export function spatialGraph(body:DirectBody,o:LighteningOptions):SpatialGraphResult{
 return callGeometryRust<SpatialGraphResult>('cad_spatial_graph',{mesh:body.mesh,options:o})
}

export function lightenSolid(body:DirectBody,o:LighteningOptions):DirectBody{
 if(![o.cell,o.rib,o.rim,o.bottom,o.top,o.seed,o.jitter,o.lineWidth,o.perimeters].every(Number.isFinite)||o.cell<=0||o.rib<=0||o.rib>=o.cell/2||o.rim<0||o.bottom<0||o.top<0||o.jitter<0||o.jitter>1||!Number.isInteger(o.seed)||o.lineWidth<=0||!Number.isInteger(o.perimeters)||o.perimeters<1||o.perimeters>8)throw Error('Check cell size, rib width, borders, seed and print settings. Rib must be smaller than half a cell.')
 if(o.rib+1e-6<o.lineWidth*o.perimeters)throw Error('Rib is thinner than the requested number of extrusion lines. Increase rib width or change the print settings.')
 return callGeometryRust<DirectBody>('cad_lightening',{body,...o})
}
