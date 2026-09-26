import { bodyPoints, type DirectBody } from './directModeling'
import { cross3, type Vec3 } from './directSketchGeometry'
import { solidTopology } from './directSolidTools'
import { evaluateNurbsCurve } from './nurbsCurve'
import type { SnapGeometry } from './modelingSnaps'

const cache=new WeakMap<DirectBody,SnapGeometry>()
/** Stable topological points, independent of the density of the display triangles. */
export function bodySnapGeometry(body: DirectBody): SnapGeometry {
  const cached=cache.get(body);if(cached)return cached
  const geometry:SnapGeometry={points:[],segments:[]},points=bodyPoints(body) as Vec3[]
  const add=(point:Vec3,kind:SnapGeometry['points'][number]['kind'])=>{
    if(point.every(Number.isFinite)&&!geometry.points.some(p=>p.kind===kind&&Math.hypot(...p.point.map((v,i)=>v-point[i]))<1e-7))geometry.points.push({point,kind})
  }
  const topology=solidTopology(body.mesh)
  if(body.brep){
    const supports=body.brep.faces.map(face=>{
      const points=face.surface.controlPoints.flat(),origin=points[0]
      let normal:Vec3|undefined
      for(let i=1;i<points.length&&!normal;i++)for(let j=i+1;j<points.length&&!normal;j++){
        const cross=cross3(points[i].map((x,k)=>x-origin[k]) as Vec3,points[j].map((x,k)=>x-origin[k]) as Vec3),length=Math.hypot(...cross)
        if(length>1e-10)normal=cross.map(v=>v/length) as Vec3
      }
      return normal&&points.every(p=>Math.abs(p.reduce((sum,v,k)=>sum+(v-origin[k])*normal![k],0))<1e-6)?{origin,normal}:null
    })
    const adjacent=body.brep.edges.map(()=>[] as number[])
    body.brep.faces.forEach((face,i)=>{for(const loop of [face.outer,...face.holes])for(const coedge of body.brep!.loops[loop].coedges)adjacent[coedge.edge].push(i)})
    for(const [index,edge] of body.brep.edges.entries()){
      const faces=adjacent[index],support=supports[faces[0]]
      // Boolean splitting may retain coplanar internal edges. They are not
      // feature edges and must not produce false vertices or midpoint snaps.
      if(faces.length>=2&&support&&faces.every(i=>{const other=supports[i];return other&&Math.abs(Math.abs(other.normal.reduce((sum,v,k)=>sum+v*support.normal[k],0))-1)<1e-7&&Math.abs(other.origin.reduce((sum,v,k)=>sum+(v-support.origin[k])*support.normal[k],0))<1e-6}))continue
      if(edge.degenerate)continue
      const curve=edge.curve,a=curve.knots[curve.degree],b=curve.knots[curve.controlPoints.length]
      const evaluate=(t:number)=>evaluateNurbsCurve(curve,a+(b-a)*t).point as Vec3
      const first=evaluate(0),middle=evaluate(.5),last=evaluate(1)
      add(first,'vertex');add(last,'vertex');add(middle,'midpoint')
      if(curve.degree===1)geometry.segments.push({a:first,b:last})
      else{
        // Each segment carries a curve evaluator: the chosen target remains on
        // the authored curve rather than on a tessellation chord.
        const samples=Array.from({length:17},(_,i)=>evaluate(i/16))
        for(let i=0;i<16;i++)geometry.segments.push({a:samples[i],b:samples[i+1],evaluate:t=>evaluate((i+t)/16)})
        if(curve.degree===2&&curve.controlPoints.length===3){
          const u=middle.map((v,i)=>v-first[i]) as Vec3,v=last.map((x,i)=>x-first[i]) as Vec3,n=cross3(u,v),n2=n.reduce((s,x)=>s+x*x,0)
          if(n2>1e-16){
            const uu=u.reduce((s,x)=>s+x*x,0),vv=v.reduce((s,x)=>s+x*x,0),vn=cross3(v,n),nu=cross3(n,u)
            const center=first.map((x,i)=>x+(uu*vn[i]+vv*nu[i])/(2*n2)) as Vec3,r=Math.hypot(...first.map((x,i)=>x-center[i]))
            if(samples.every(p=>Math.abs(Math.hypot(...p.map((x,i)=>x-center[i]))-r)<Math.max(1e-7,r*1e-7)))add(center,'center')
          }
        }
      }
    }
  }else{
    // Omit triangle-fan interior vertices and coplanar diagonals.
    for(const edge of topology.edges){
      const a=points[edge.a],b=points[edge.b]
      add(a,'vertex');add(b,'vertex');add(a.map((v,i)=>(v+b[i])/2) as Vec3,'midpoint');geometry.segments.push({a,b})
    }
  }
  for(const face of topology.faces){
    if(body.brep&&!body.brep.faces.some(f=>f.surface.controlPoints.flat().every(p=>Math.abs(p.reduce((sum,v,i)=>sum+(v-face.center[i])*face.normal[i],0))<Math.max(1e-6,body.brep!.toleranceMm*10))))continue
    const center:Vec3=[0,0,0];let area=0
    for(const index of face.triangles){
      const [a,b,c]=Array.from(body.mesh.indices.slice(index*3,index*3+3),i=>points[i]),u=b.map((v,i)=>v-a[i]) as Vec3,v=c.map((x,i)=>x-a[i]) as Vec3,w=Math.hypot(...cross3(u,v))
      area+=w;for(let i=0;i<3;i++)center[i]+=(a[i]+b[i]+c[i])*w/3
    }
    if(area>1e-12)add(center.map(v=>v/area) as Vec3,'center')
  }
  if(points.length)add([0,1,2].map(i=>{let min=Infinity,max=-Infinity;for(const p of points){min=Math.min(min,p[i]);max=Math.max(max,p[i])}return(min+max)/2}) as Vec3,'bounds-center')
  cache.set(body,geometry);return geometry
}
