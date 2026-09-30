/** A vertex starts with xyz; remaining linear attributes are interpolated at cuts. */
export type TransparentVertex = readonly number[]
export type TransparentTriangle = readonly [TransparentVertex, TransparentVertex, TransparentVertex]

/** Split against n·xyz+d=0. Input and output attributes are never aliased. */
export function splitTransparentTriangle(triangle: TransparentTriangle, plane: readonly [number,number,number,number], tolerance=0) {
  const width=triangle[0].length, length=Math.hypot(...plane.slice(0,3))
  if(width<3||triangle.some(v=>v.length!==width||v.some(x=>!Number.isFinite(x)))||plane.some(x=>!Number.isFinite(x))||!Number.isFinite(length)||length===0||!Number.isFinite(tolerance)||tolerance<0)throw Error('Invalid transparent triangle split')
  const distances=triangle.map(v=>(plane[0]*v[0]+plane[1]*v[1]+plane[2]*v[2]+plane[3])/length)
  if(distances.some(x=>!Number.isFinite(x)))throw Error('Unrepresentable transparent triangle distance')
  // Bound cancellation error of the dot product instead of merging distinct layers
  // with a fixed model-space epsilon. Explicit caller tolerances remain supported.
  const signs=distances.map((d,i)=>{
    const v=triangle[i],roundoff=16*Number.EPSILON*(Math.abs(plane[0]*v[0])+Math.abs(plane[1]*v[1])+Math.abs(plane[2]*v[2])+Math.abs(plane[3]))/length
    return Math.abs(d)<=Math.max(tolerance,roundoff)?0:Math.sign(d)
  })
  const clone=()=>triangle.map(v=>[...v]) as [number[],number[],number[]]
  const front: number[][][]=[],back:number[][][]=[],coplanar:number[][][]=[]
  if(signs.every(s=>s===0))coplanar.push(clone())
  else if(signs.every(s=>s>=0))front.push(clone())
  else if(signs.every(s=>s<=0))back.push(clone())
  else {
    const positive:number[][]=[],negative:number[][]=[]
    for(let i=0;i<3;i++){
      const j=(i+1)%3,a=triangle[i],b=triangle[j]
      if(signs[i]>=0)positive.push([...a])
      if(signs[i]<=0)negative.push([...a])
      if(signs[i]*signs[j]<0){
        const t=distances[i]/(distances[i]-distances[j])
        const point=a.map((value,k)=>value*(1-t)+b[k]*t)
        positive.push(point);negative.push([...point])
      }
    }
    for(const [polygon,result] of [[positive,front],[negative,back]] as const)
      for(let i=1;i+1<polygon.length;i++)result.push([[...polygon[0]],[...polygon[i]],[...polygon[i+1]]])
  }
  return {front,back,coplanar}
}
