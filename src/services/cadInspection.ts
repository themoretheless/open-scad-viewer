import type {DirectBody} from './directModeling'
import {callGeometryRust} from './geometry/kernel'
export interface CadPairReport {a:string;b:string;overlapMm3:number;gapMm:number}
/** Native bounded display-mesh clearance, not certified analytic B-rep distance. */
export function inspectCadPairs(bodies:DirectBody[]):CadPairReport[]{
 return callGeometryRust('cad_inspect_pairs',{bodies})
}
