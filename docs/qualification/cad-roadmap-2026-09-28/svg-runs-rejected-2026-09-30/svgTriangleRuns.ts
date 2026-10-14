/** Merge only consecutive opaque triangles with the same paint. Their order
 * relative to every other body remains unchanged. Unmergeable entries retain
 * their original identity and representation. */
export function svgTriangleRuns<T extends {points:string}>(items:readonly T[],paint:(item:T)=>string|null):Array<T&{path?:string}>{
 const out:Array<T&{path?:string}>=[]
 let previous:string|null=null
 for(const item of items){
  const key=paint(item),last=out.at(-1)
  if(key!==null&&key===previous&&last){
   last.path=(last.path??'M'+last.points.replaceAll(' ','L')+'Z')+'M'+item.points.replaceAll(' ','L')+'Z'
  }else out.push({...item})
  previous=key
 }
 return out
}
