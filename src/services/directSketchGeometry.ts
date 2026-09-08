import type { DirectSketch, Point2 } from './directModeling'
export type Vec3 = [number,number,number]
export interface SketchPlane { origin:Vec3; u:Vec3; v:Vec3 }
export interface AnalyticCurve { kind:'circle'|'arc'; center:Point2; radius:number; start:number; sweep:number }
export const dot3=(a:number[],b:number[])=>a.reduce((s,v,i)=>s+v*b[i],0)
export const cross3=(a:number[],b:number[]):Vec3=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
export const unit3=(a:number[]):Vec3=>{const n=Math.hypot(...a);if(n<1e-9)throw Error('Zero direction');return a.map(v=>v/n) as Vec3}
export const xyPlane=():SketchPlane=>({origin:[0,0,0],u:[1,0,0],v:[0,1,0]})
export function worldPoint(p:number[],plane:SketchPlane=xyPlane()):Vec3 {const n=cross3(plane.u,plane.v);return plane.origin.map((v,i)=>v+p[0]*plane.u[i]+p[1]*plane.v[i]+(p[2]??0)*n[i]) as Vec3}
export function sampleCurve(c:AnalyticCurve):Point2[] {
 if(!['circle','arc'].includes(c.kind)||!Array.isArray(c.center)||c.center.length!==2||!c.center.every(Number.isFinite)||![c.radius,c.start,c.sweep].every(Number.isFinite)||c.radius<.01||c.radius>1e6||Math.abs(c.sweep)<.1||Math.abs(c.sweep)>360)throw Error('Invalid circle or arc parameters.')
 const sweep=c.kind==='circle'?360:c.sweep,n=Math.max(8,Math.ceil(Math.abs(sweep)/3)),count=c.kind==='circle'?n:n+1
 return Array.from({length:count},(_,i)=>{const a=(c.start+sweep*i/n)*Math.PI/180;return [c.center[0]+c.radius*Math.cos(a),c.center[1]+c.radius*Math.sin(a)]})
}
export function bakeSketch(s:DirectSketch):DirectSketch {const next=structuredClone(s);delete next.analytic;return next}
export function transformSketch(s:DirectSketch,delta:Point2,angle:number,scale:number,pivot?:Point2):DirectSketch {
 if(![...delta,angle,scale].every(Number.isFinite)||scale<=0)throw Error('Invalid sketch transform.')
 const next=structuredClone(s),center=pivot??(s.analytic?.center??[s.points.reduce((v,p)=>v+p[0],0)/s.points.length,s.points.reduce((v,p)=>v+p[1],0)/s.points.length]),a=angle*Math.PI/180,c=Math.cos(a),sn=Math.sin(a)
 const apply=(p:Point2):Point2=>[center[0]+delta[0]+scale*((p[0]-center[0])*c-(p[1]-center[1])*sn),center[1]+delta[1]+scale*((p[0]-center[0])*sn+(p[1]-center[1])*c)]
 next.points=next.points.map(apply)
 if(next.analytic){next.analytic.center=apply(next.analytic.center);next.analytic.radius*=scale;next.analytic.start+=angle;next.points=sampleCurve(next.analytic)}
 return next
}
export function validateSimpleSketch(points:Point2[],closed=true) {
 const n=points.length,edges=closed?n:n-1
 if(n<2||n>512||!points.every(p=>p.every(Number.isFinite)))throw Error('Invalid contour.')
 const cross=(a:Point2,b:Point2,c:Point2)=>(b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])
 for(let i=0;i<edges;i++)for(let j=i+2;j<edges;j++){
  if(closed&&i===0&&j===n-1)continue
  const a=points[i],b=points[(i+1)%n],c=points[j],d=points[(j+1)%n]
  if(Math.max(a[0],b[0])<Math.min(c[0],d[0])-1e-8||Math.max(c[0],d[0])<Math.min(a[0],b[0])-1e-8||Math.max(a[1],b[1])<Math.min(c[1],d[1])-1e-8||Math.max(c[1],d[1])<Math.min(a[1],b[1])-1e-8)continue
  if(cross(a,b,c)*cross(a,b,d)<=0&&cross(c,d,a)*cross(c,d,b)<=0)throw Error('The contour would self-intersect.')
 }
}
export function offsetSketch(s:DirectSketch,distance:number):DirectSketch {
 if(!Number.isFinite(distance)||Math.abs(distance)<.001)throw Error('Enter a nonzero offset.')
 if(s.analytic){const next=structuredClone(s);next.analytic!.radius+=distance;next.points=sampleCurve(next.analytic!);return next}
 if(!s.closed)throw Error('Offset requires a closed contour or an analytic arc.')
 validateSimpleSketch(s.points)
 const p=s.points,n=p.length,area=p.reduce((v,a,i)=>{const b=p[(i+1)%n];return v+a[0]*b[1]-a[1]*b[0]},0),sign=Math.sign(area)
 if(Math.abs(area)<1e-8)throw Error('Degenerate contour.')
 const lines=p.map((a,i)=>{const b=p[(i+1)%n],d:Point2=[b[0]-a[0],b[1]-a[1]],l=Math.hypot(...d);if(l<1e-8)throw Error('Zero edge.');return {a:[a[0]+sign*distance*d[1]/l,a[1]-sign*distance*d[0]/l] as Point2,d}})
 const points=lines.map((b,i):Point2=>{const a=lines[(i+n-1)%n],det=a.d[0]*b.d[1]-a.d[1]*b.d[0];if(Math.abs(det)<1e-9){if(dot3([...a.d,0],[...b.d,0])<0)throw Error('Folded contour.');return b.a}const t=((b.a[0]-a.a[0])*b.d[1]-(b.a[1]-a.a[1])*b.d[0])/det;return [a.a[0]+t*a.d[0],a.a[1]+t*a.d[1]]})
 validateSimpleSketch(points)
 // Reject collapsed offsets, including an inset which passed through the opposite edge.
 for(let i=0;i<n;i++){const a=points[i],b=points[(i+1)%n],old=lines[i].d;if((b[0]-a[0])*old[0]+(b[1]-a[1])*old[1]<=1e-8)throw Error('Offset collapses an edge. Reduce the distance.')}
 return {...bakeSketch(s),points}
}
function intersection(a:Point2,b:Point2,c:Point2,d:Point2):{t:number;u:number}|null {
 const x=b[0]-a[0],y=b[1]-a[1],vx=d[0]-c[0],vy=d[1]-c[1],det=x*vy-y*vx
 if(Math.abs(det)<1e-10)return null
 return {t:((c[0]-a[0])*vy-(c[1]-a[1])*vx)/det,u:((c[0]-a[0])*y-(c[1]-a[1])*x)/det}
}
/** Trim a clicked segment between its closest intersections, retaining both remaining chains. */
export function trimSketch(s:DirectSketch,edge:number,at:number,boundaries:DirectSketch[]):DirectSketch[] {
 const p=s.points,n=p.length;if(!Number.isInteger(edge)||edge<0||edge>=(s.closed?n:n-1))throw Error('Select an edge to trim.')
 const a=p[edge],b=p[(edge+1)%n],cuts=[0,1]
 for(const other of boundaries)for(let i=0;i<(other.closed?other.points.length:other.points.length-1);i++){
  if(other.id===s.id&&i===edge)continue
  const hit=intersection(a,b,other.points[i],other.points[(i+1)%other.points.length]);if(hit&&hit.t>1e-8&&hit.t<1-1e-8&&hit.u>=0&&hit.u<=1)cuts.push(hit.t)
 }
 cuts.sort((a,b)=>a-b);const hi=cuts.find(t=>t>at+1e-8)??1,lo=[...cuts].reverse().find(t=>t<at-1e-8)??0
 const point=(t:number):Point2=>[a[0]+t*(b[0]-a[0]),a[1]+t*(b[1]-a[1])]
 let chains:Point2[][]
 if(s.closed){const chain=[point(hi)];for(let k=1;k<=n;k++)chain.push(p[(edge+k)%n]);chain.push(point(lo));chains=[chain]}
 else chains=[[...p.slice(0,edge+1),point(lo)],[point(hi),...p.slice(edge+1)]]
 return chains.map(points=>points.filter((v,i)=>!i||Math.hypot(v[0]-points[i-1][0],v[1]-points[i-1][1])>1e-8)).filter(p=>p.length>=2).map((points,i)=>({...bakeSketch(s),id:i?s.id+'-trim':s.id,closed:false,points}))
}
export function extendSketch(s:DirectSketch,end:'start'|'end',boundaries:DirectSketch[]):DirectSketch {
 if(s.closed||s.analytic)throw Error('Extend requires an open polyline.')
 const points=s.points.map(p=>[...p] as Point2),i=end==='start'?0:points.length-1,j=end==='start'?1:points.length-2,a=points[j],b=points[i];let best=Infinity
 for(const other of boundaries)for(let k=0;k<(other.closed?other.points.length:other.points.length-1);k++){
  if(other.id===s.id)continue
  const hit=intersection(a,b,other.points[k],other.points[(k+1)%other.points.length]);if(hit&&hit.t>1+1e-8&&hit.t<best&&hit.u>=0&&hit.u<=1)best=hit.t
 }
 if(!Number.isFinite(best))throw Error('No boundary intersects the extended ray.')
 points[i]=[a[0]+best*(b[0]-a[0]),a[1]+best*(b[1]-a[1])];validateSimpleSketch(points,false);return {...s,points}
}
