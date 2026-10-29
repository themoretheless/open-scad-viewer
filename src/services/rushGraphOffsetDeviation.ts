import {inspectNurbsOffsetDeviation,type OffsetDeviationOptions} from './nurbsOffsetDeviation'
import type {Value} from './rushGraphNurbsKernel'
export interface RushOffsetDeviationRequest extends Omit<OffsetDeviationOptions,'source'|'candidate'> {
 source:string
 candidate:string
}
/** Resolve graph identities; all numerical geometry remains in Rust. */
export function inspectRushGraphOffsetDeviation(request:RushOffsetDeviationRequest,get:(id:string)=>Value) {
 const source=get(request.source),candidate=get(request.candidate)
 if(source.kind!=='surface'||candidate.kind!=='surface')throw new Error('Offset deviation requires two surface nodes.')
 return inspectNurbsOffsetDeviation({...request,source:source.data,candidate:candidate.data})
}
