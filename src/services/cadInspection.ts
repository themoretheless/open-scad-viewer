import {bodyPoints,type DirectBody} from './directModeling'
import {booleanPolygonMeshes} from './polygonKernel'
const sub=(a:number[],b:number[])=>a.map((v,k)=>v-b[k]),dot=(a:number[],b:number[])=>a.reduce((s,v,k)=>s+v*b[k],0)
const norm=(v:number[])=>Math.sqrt(dot(v,v))
function pointTriangle(p:number[],a:number[],b:number[],c:number[]){
 const ab=sub(b,a),ac=sub(c,a),ap=sub(p,a),d1=dot(ab,ap),d2=dot(ac,ap)
 if(d1<=0&&d2<=0)return norm(ap)
 const bp=sub(p,b),d3=dot(ab,bp),d4=dot(ac,bp);if(d3>=0&&d4<=d3)return norm(bp)
 const vc=d1*d4-d3*d2;if(vc<=0&&d1>=0&&d3<=0)return norm(sub(ap,ab.map(v=>v*d1/(d1-d3))))
 const cp=sub(p,c),d5=dot(ab,cp),d6=dot(ac,cp);if(d6>=0&&d5<=d6)return norm(cp)
 const vb=d5*d2-d1*d6;if(vb<=0&&d2>=0&&d6<=0)return norm(sub(ap,ac.map(v=>v*d2/(d2-d6))))
 const va=d3*d6-d5*d4;if(va<=0&&d4-d3>=0&&d5-d6>=0)return norm(sub(bp,sub(c,b).map(v=>v*(d4-d3)/(d4-d3+d5-d6))))
 const sum=va+vb+vc;return norm(sub(ap,ab.map((v,k)=>(v*vb+ac[k]*vc)/sum)))
}
function segments(a:number[],b:number[],c:number[],d:number[]){
 const u=sub(b,a),v=sub(d,c),w=sub(a,c),aa=dot(u,u),bb=dot(u,v),cc=dot(v,v),dd=dot(u,w),ee=dot(v,w),den=aa*cc-bb*bb
 let s=den>1e-20?Math.max(0,Math.min(1,(bb*ee-cc*dd)/den)):0,t=(bb*s+ee)/cc
 if(t<0){t=0;s=Math.max(0,Math.min(1,-dd/aa))}else if(t>1){t=1;s=Math.max(0,Math.min(1,(bb-dd)/aa))}
 return norm(w.map((x,k)=>x+s*u[k]-t*v[k]))
}
export interface CadPairReport {a:string;b:string;overlapMm3:number;gapMm:number}
/** Exhaustive triangle distance, including edge-edge minima. Work budget is explicit. */
export function inspectCadPairs(bodies:DirectBody[]):CadPairReport[]{
 if(bodies.length<2)throw Error('Select at least two bodies.')
 let budget=0;for(let i=0;i<bodies.length;i++)for(let j=i+1;j<bodies.length;j++)budget+=bodies[i].mesh.indices.length*bodies[j].mesh.indices.length/9
 if(budget>2_000_000)throw Error('Clearance check exceeds two million triangle pairs; select fewer/simpler bodies.')
 const triangles=bodies.map(b=>{const p=bodyPoints(b);return Array.from({length:b.mesh.indices.length/3},(_,i)=>b.mesh.indices.slice(i*3,i*3+3).map(j=>p[j]))}),out:CadPairReport[]=[]
 for(let i=0;i<bodies.length;i++)for(let j=i+1;j<bodies.length;j++){
  const intersection=booleanPolygonMeshes(bodies[i].mesh,bodies[j].mesh,'intersection'),volume=Math.abs(intersection.report.signedVolumeMm3)
  let gap=volume>1e-9?0:Infinity
  if(gap)for(const a of triangles[i])for(const b of triangles[j]){
   for(const p of a)gap=Math.min(gap,pointTriangle(p,b[0],b[1],b[2]));for(const p of b)gap=Math.min(gap,pointTriangle(p,a[0],a[1],a[2]));for(let x=0;x<3;x++)for(let y=0;y<3;y++)gap=Math.min(gap,segments(a[x],a[(x+1)%3],b[y],b[(y+1)%3]))
  }
  out.push({a:bodies[i].name,b:bodies[j].name,overlapMm3:volume,gapMm:gap})
 }return out
}
