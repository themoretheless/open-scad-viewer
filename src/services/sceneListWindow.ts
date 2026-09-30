/** Pixel offsets for a scene list with fixed object rows and taller group headers.
 * Geometry stays in the document; only the range of rendered rows changes. */
export interface SceneListWindow {start:number;end:number;before:number;after:number;total:number}
export function sceneRowOffsets(heights:readonly number[],gap=1):number[]{
 if(!Number.isFinite(gap)||gap<0)throw Error('Invalid scene row gap.')
 const offsets=[0]
 for(const height of heights){
  if(!Number.isFinite(height)||height<=0)throw Error('Invalid scene row height.')
  offsets.push(offsets[offsets.length-1]+height+gap)
 }
 return offsets
}
export function sceneListWindow(offsets:readonly number[],top:number,height:number,overscan=8):SceneListWindow{
 if(!Number.isFinite(top)||!Number.isFinite(height)||height<0||!Number.isInteger(overscan)||overscan<0)throw Error('Invalid scene viewport.')
 const count=Math.max(0,offsets.length-1),total=offsets[count]??0
 const locate=(pixel:number)=>{
  let lo=0,hi=count
  while(lo<hi){const mid=(lo+hi+1)>>>1;if(offsets[mid]<=pixel)lo=mid;else hi=mid-1}
  return lo
 }
 const bounded=Math.max(0,Math.min(top,total))
 const start=Math.max(0,locate(bounded)-overscan)
 const end=Math.min(count,locate(Math.min(total,bounded+height))+1+overscan)
 return {start,end,before:offsets[start]??0,after:total-(offsets[end]??0),total}
}
