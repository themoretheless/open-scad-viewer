/** Frozen viewer-subset quality rule from the former TS evaluator. */
export function referenceLegacySegments(raw:number|null,fallback:number,minimum:number,quality:'preview'|'full'){
 const requested=raw===null?null:Math.round(raw),maximum=quality==='preview'?48:256
 const beforeCap=requested??(quality==='preview'?Math.min(fallback,24):fallback)
 let segments=beforeCap
 const clamped=segments>maximum
 if(clamped)segments=maximum
 segments=Math.max(minimum,segments)
 const reduced=quality==='preview'&&segments!==Math.max(minimum,Math.min(requested??fallback,256))
 return {requested,beforeCap,maximum,segments,clamped,reduced}
}
