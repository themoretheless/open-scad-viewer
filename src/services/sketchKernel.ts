import {callGeometryRust} from './geometryRustKernel'
export type SketchConstraint =
 | {kind:'fix';point:number;at:number[]}
 | {kind:'horizontal'|'vertical'|'coincident';a:number;b:number}
 | {kind:'distance';a:number;b:number;value:number}
 | {kind:'parallel'|'perpendicular'|'equal_length';a:number;b:number;c:number;d:number}
 | {kind:'radius';circle:number;value:number}
 | {kind:'point_on_circle';point:number;circle:number}
 | {kind:'tangent_line_circle';a:number;b:number;circle:number}
 | {kind:'tangent_circles';a:number;b:number;internal:boolean}
export interface NativeSketch {points:number[][];circles?:{center:number;radius:number}[];constraints:SketchConstraint[]}
export interface SketchSolution {sketch:NativeSketch;status:'solved'|'underconstrained'|'not_converged'|'degenerate';iterations:number;maxResidual:number;residuals:number[][];degreesOfFreedom:number;convergenceCertified:false}
export const solveNativeSketch=(sketch:NativeSketch,tolerance=1e-6):SketchSolution=>callGeometryRust('sketch_solve',{sketch,tolerance})
