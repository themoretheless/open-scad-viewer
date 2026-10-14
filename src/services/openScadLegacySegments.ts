/** Native viewer-subset subdivision rules shared by evaluators and semantic plans. */
import {languageRequest} from './languages/kernel'
export interface LegacySegmentResolution {
  requested:number|null;beforeCap:number;maximum:number;segments:number;clamped:boolean;reduced:boolean
}
export function resolveLegacyOpenScadSegments(requested:number|null,fallback:number,minimum:number,quality:'preview'|'full'):LegacySegmentResolution {
  const response=languageRequest(15,{requested,fallback,minimum,preview:quality==='preview'}) as {ok:boolean;value:LegacySegmentResolution;error?:{message:string}}
  if(!response.ok)throw new Error(response.error?.message??'Native legacy subdivision failed')
  return response.value
}
export function roundLegacyOpenScadSegments(value:number):number {
  return resolveLegacyOpenScadSegments(value,0,0,'full').requested!
}
