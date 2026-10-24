// Transport validation; Rust owns the complete actual-neighbor G1 proof.
import {validSupportTangencyRequest,type SupportTangencyRequest} from './expressionSupportTangency'
export interface NeighborTangencyRequest extends SupportTangencyRequest {maxSineSquared:number}
export interface NeighborTangencyReport {
 version:1;qualified:boolean;basis:'owned-full-Hermite-actual-shell-neighbors'
 upperSineSquared:[number,number,number,number]|null
 cells:number;spans:number;reason:string;solidQualified:false
}
export function validNeighborTangencyRequest(value:unknown):value is NeighborTangencyRequest {
 if(!validSupportTangencyRequest(value))return false
 const t=(value as NeighborTangencyRequest).maxSineSquared
 return Number.isFinite(t)&&t>=0&&t<1
}
export function validNeighborTangencyReport(request:NeighborTangencyRequest,value:unknown):value is NeighborTangencyReport {
 if(!value||typeof value!=='object')return false
 const v=value as NeighborTangencyReport
 if(v.version!==1||v.basis!=='owned-full-Hermite-actual-shell-neighbors'||v.solidQualified!==false
  ||typeof v.qualified!=='boolean'||typeof v.reason!=='string'||!v.reason
  ||!Number.isSafeInteger(v.cells)||v.cells<0||v.cells>request.cells
  ||!Number.isSafeInteger(v.spans)||v.spans<0||v.spans>request.spans)return false
 const qualifiedReason='profile-transition-neighbor-tangency-qualified'
 if(!v.qualified)return v.upperSineSquared===null&&v.reason!==qualifiedReason
 if(v.reason!==qualifiedReason)return false
 if(request.controls===0||request.cells===0||request.spans===0||v.cells===0||v.spans===0)return false
 return Array.isArray(v.upperSineSquared)&&v.upperSineSquared.length===4
  &&v.upperSineSquared.every(x=>Number.isFinite(x)&&x>=0&&x<=request.maxSineSquared)
  &&v.upperSineSquared[0]===0&&v.upperSineSquared[1]===0
}
