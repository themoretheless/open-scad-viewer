// Transport checks only; native code owns radius bounds and qualification.
export interface ExpressionRadiusRequest {
 centers:unknown;radius:unknown;lawDomains:[[number,number],[number,number]]
 toleranceMm:number;controls:number;spans:number;coefficientWork:number
}
export interface ExpressionRadiusReport {
 version:1;qualified:boolean;violationQualified:boolean;solidQualified:false
 errorLowerMm:number|null;errorUpperMm:number|null;spans:number;coefficientWork:number;reason:string
}
const finite=(v:unknown):v is number=>typeof v==='number'&&Number.isFinite(v)
const count=(v:unknown,max:number)=>Number.isSafeInteger(v)&&(v as number)>=0&&(v as number)<=max
function curve(value:unknown,dimensions:number):boolean {
 if(!value||typeof value!=='object')return false
 const c=value as {degree:number;knots:number[];controlPoints:number[][];weights:number[];periodic:boolean}
 return count(c.degree,64)&&Array.isArray(c.controlPoints)&&c.controlPoints.length>c.degree&&c.controlPoints.length<=100000
  &&c.controlPoints.every(p=>Array.isArray(p)&&p.length===dimensions&&p.every(finite))
  &&Array.isArray(c.knots)&&c.knots.length===c.controlPoints.length+c.degree+1&&c.knots.every(finite)
  &&Array.isArray(c.weights)&&c.weights.length===c.controlPoints.length&&c.weights.every(finite)&&typeof c.periodic==='boolean'
}
export function validExpressionRadiusRequest(value:unknown):value is ExpressionRadiusRequest {
 if(!value||typeof value!=='object')return false
 const v=value as ExpressionRadiusRequest
 return curve(v.centers,3)&&curve(v.radius,2)&&finite(v.toleranceMm)&&v.toleranceMm>0
  &&count(v.controls,1000000)&&count(v.spans,100000)&&count(v.coefficientWork,100000000)
  &&Array.isArray(v.lawDomains)&&v.lawDomains.length===2&&v.lawDomains.every(d=>Array.isArray(d)&&d.length===2&&d.every(finite)&&d[0]<d[1])
}
export function validExpressionRadiusReport(request:ExpressionRadiusRequest,value:unknown):value is ExpressionRadiusReport {
 if(!value||typeof value!=='object')return false
 const v=value as ExpressionRadiusReport
 if(v.version!==1||v.solidQualified!==false||typeof v.qualified!=='boolean'||typeof v.violationQualified!=='boolean'
  ||typeof v.reason!=='string'||!v.reason||!count(v.spans,request.spans)||!count(v.coefficientWork,request.coefficientWork))return false
 for(const error of [v.errorLowerMm,v.errorUpperMm])if(error!==null&&(!finite(error)||error<0))return false
 if(v.errorLowerMm!==null&&(v.errorUpperMm===null||v.errorLowerMm>v.errorUpperMm))return false
 if((request.controls===0||request.spans===0||request.coefficientWork===0)
  &&(v.errorLowerMm!==null||v.errorUpperMm!==null))return false
 return v.qualified===(v.errorUpperMm!==null&&v.errorUpperMm<=request.toleranceMm)
  &&v.violationQualified===(v.errorLowerMm!==null&&v.errorLowerMm>request.toleranceMm)
}
