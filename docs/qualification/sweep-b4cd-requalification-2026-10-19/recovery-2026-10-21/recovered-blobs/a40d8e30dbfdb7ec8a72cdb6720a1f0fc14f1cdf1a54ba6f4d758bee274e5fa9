/** Saved loop membership at construction; edits require fresh geometry checks. */
export interface CurveOffsetRegion {
 version:1;scope:'at-construction';sourceId:string;loopId:string;part:number;parts:number;fillRule:'nonzero'|'evenodd'
 originalOffsetTopologyCertified:false
}
export function validCurveOffsetRegion(value:unknown):value is CurveOffsetRegion {
 if(!value||typeof value!=='object')return false
 const v=value as CurveOffsetRegion
 const id=(s:unknown)=>typeof s==='string'&&s.length>0&&s.length<=1024
 return v.version===1&&v.scope==='at-construction'&&id(v.sourceId)&&id(v.loopId)
 &&Number.isInteger(v.parts)&&v.parts>=1&&v.parts<=128&&Number.isInteger(v.part)&&v.part>=0&&v.part<v.parts
 &&['nonzero','evenodd'].includes(v.fillRule)&&v.originalOffsetTopologyCertified===false
}
