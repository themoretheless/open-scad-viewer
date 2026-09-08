/** Shared parameters only; each kernel edits its own representation. */
export type GeometryDeformation =
 | {kind:'twist';origin:number[];radians_per_unit:number}
 | {kind:'bend';origin:number[];radius:number}
 | {kind:'lattice';min:number[];max:number[];controls:number[][]}
export interface GeometryBrush {center:number[];radius:number;displacement:number[]}
