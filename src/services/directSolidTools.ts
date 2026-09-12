import { booleanPolygonMeshes, extrudePolygonProfile, inspectPolygonMesh, type PolygonMesh } from './geometry/polygon'
import { bodyPoints, type DirectBody } from './directModeling'
import { cross3, dot3, unit3, worldPoint, type Vec3, type SketchPlane } from './directSketchGeometry'
export interface SolidFace { triangles:number[]; normal:Vec3; offset:number; vertices:number[]; center:Vec3 }
export interface SolidEdge { a:number; b:number; faces:[number,number] }
const sub=(a:number[],b:number[])=>a.map((v,i)=>v-b[i]) as Vec3
const add=(a:number[],b:number[])=>a.map((v,i)=>v+b[i]) as Vec3
const mul=(a:number[],s:number)=>a.map(v=>v*s) as Vec3
const polygonBody=(body:DirectBody,mesh:PolygonMesh):DirectBody=>{const next={...body,mesh};delete next.brep;return next}
export function solidTopology(mesh:PolygonMesh):{faces:SolidFace[];edges:SolidEdge[]} {
 const p=Array.from({length:mesh.positions.length/3},(_,i)=>mesh.positions.slice(i*3,i*3+3) as Vec3),faces:SolidFace[]=[],triangleFace:number[]=[]
 for(let i=0;i<mesh.indices.length;i+=3){const ids=mesh.indices.slice(i,i+3),[a,b,c]=ids.map(i=>p[i]),raw=cross3(sub(b,a),sub(c,a));if(Math.hypot(...raw)<1e-9)continue
  const normal=unit3(raw),offset=dot3(normal,a);let f=faces.findIndex(f=>dot3(f.normal,normal)>1-1e-8&&Math.abs(f.offset-offset)<1e-6)
  if(f<0){f=faces.length;faces.push({triangles:[],normal,offset,vertices:[],center:[0,0,0]})}
  faces[f].triangles.push(i/3);faces[f].vertices.push(...ids);triangleFace[i/3]=f
 }
 for(const f of faces){f.vertices=[...new Set(f.vertices)];f.center=[0,1,2].map(k=>f.vertices.reduce((s,i)=>s+p[i][k],0)/f.vertices.length) as Vec3}
 // Coordinate keys join seams in meshes that don't share vertex indices.
 const key=(i:number)=>p[i].map(v=>v.toFixed(7)).join(','),edgeMap=new Map<string,{a:number;b:number;faces:number[]}>()
 for(let t=0;t<mesh.indices.length/3;t++){const ids=mesh.indices.slice(t*3,t*3+3);for(let k=0;k<3;k++){let a=ids[k],b=ids[(k+1)%3];if(key(a)>key(b))[a,b]=[b,a];const id=key(a)+'|'+key(b),e=edgeMap.get(id)??{a,b,faces:[]};e.faces.push(triangleFace[t]);edgeMap.set(id,e)}}
 const edges=[...edgeMap.values()].filter(e=>e.faces.length===2&&e.faces[0]!==e.faces[1]).map(e=>({a:e.a,b:e.b,faces:e.faces as [number,number]}))
 return {faces,edges}
}
export function facePlane(body:DirectBody,face:SolidFace):SketchPlane {
 const p=bodyPoints(body),origin=p[face.vertices[0]] as Vec3,u=unit3(sub(p[face.vertices.find(i=>Math.hypot(...sub(p[i],origin))>1e-7)!],origin)),v=unit3(cross3(face.normal,u))
 return {origin,u,v}
}
interface Halfspace {normal:Vec3;offset:number}
function convexPlanes(body:DirectBody):Halfspace[] {
 const report=inspectPolygonMesh(body.mesh);if(!report.closed)throw Error('This operation requires a closed solid.')
 const faces=solidTopology(body.mesh).faces,p=bodyPoints(body)
 if(faces.length>64)throw Error('This operation supports convex solids with up to 64 planar faces.')
 if(faces.some(f=>p.some(v=>dot3(f.normal,v)>f.offset+1e-5)))throw Error('This operation currently requires a convex solid with planar faces.')
 return faces.map(({normal,offset})=>({normal,offset}))
}
/** Intersect support planes, then triangulate their convex face polygons with shared vertices. */
function fromPlanes(planes:Halfspace[]):PolygonMesh {
 const points:Vec3[]=[],eps=1e-6
 for(let i=0;i<planes.length;i++)for(let j=i+1;j<planes.length;j++)for(let k=j+1;k<planes.length;k++){
  const a=planes[i],b=planes[j],c=planes[k],bc=cross3(b.normal,c.normal),det=dot3(a.normal,bc)
  if(Math.abs(det)<1e-8)continue
  const p=mul(add(add(mul(bc,a.offset),mul(cross3(c.normal,a.normal),b.offset)),mul(cross3(a.normal,b.normal),c.offset)),1/det)
  if(planes.some(f=>dot3(f.normal,p)>f.offset+eps)||points.some(q=>Math.hypot(...sub(p,q))<eps))continue
  points.push(p)
 }
 if(points.length<4)throw Error('The dimension collapses the solid. Use a smaller value.')
 const indices:number[]=[]
 for(const face of planes){const ids=points.map((p,i)=>Math.abs(dot3(face.normal,p)-face.offset)<eps?i:-1).filter(i=>i>=0);if(ids.length<3)continue
  const center=mul(ids.map(i=>points[i]).reduce((a,b)=>add(a,b),[0,0,0] as Vec3),1/ids.length),u=unit3(sub(points[ids[0]],center)),v=cross3(face.normal,u)
  ids.sort((a,b)=>Math.atan2(dot3(sub(points[a],center),v),dot3(sub(points[a],center),u))-Math.atan2(dot3(sub(points[b],center),v),dot3(sub(points[b],center),u)))
  for(let i=1;i<ids.length-1;i++)indices.push(ids[0],ids[i],ids[i+1])
 }
 const mesh={positions:points.flat(),indices},report=inspectPolygonMesh(mesh)
 if(!report.closed||report.signedVolumeMm3<1e-8)throw Error('The operation produced an invalid solid.')
 return mesh
}
export function pushPullFace(body:DirectBody,faceIndex:number,distance:number):DirectBody {
 if(!Number.isFinite(distance))throw Error('Enter a finite distance.')
 const planes=convexPlanes(body);if(!planes[faceIndex])throw Error('Select a face.')
 planes[faceIndex].offset+=distance;return polygonBody(body,fromPlanes(planes))
}
export function bevelSolidEdge(body:DirectBody,edgeIndex:number,size:number,kind:'chamfer'|'fillet'):DirectBody {
 if(!Number.isFinite(size)||size<.01)throw Error('Size must be at least 0.01 mm.')
 const planes=convexPlanes(body),topology=solidTopology(body.mesh),edge=topology.edges[edgeIndex]
 if(!edge)throw Error('Select an edge.')
 const a=planes[edge.faces[0]],b=planes[edge.faces[1]],cos=dot3(a.normal,b.normal),alpha=Math.acos(Math.max(-1,Math.min(1,cos))),p=bodyPoints(body)[edge.a]
 if(alpha<1e-4||Math.PI-alpha<1e-4)throw Error('Select a convex edge.')
 const bisector=unit3(add(a.normal,b.normal)),center=sub(p,mul(bisector,size/Math.cos(alpha/2)))
 if(kind==='chamfer')planes.push({normal:bisector,offset:dot3(bisector,p)-size*Math.sin(alpha/2)})
 else for(let i=1;i<16;i++){
  const t=i/16,normal=unit3(add(mul(a.normal,Math.sin((1-t)*alpha)),mul(b.normal,Math.sin(t*alpha))))
  planes.push({normal,offset:dot3(normal,center)+size})
 }
 const mesh=fromPlanes(planes)
 // Both adjacent support faces must survive; otherwise the chosen size consumed the feature.
 const result=solidTopology(mesh)
 if([a,b].some(f=>!result.faces.some(g=>dot3(g.normal,f.normal)>1-1e-6&&Math.abs(g.offset-f.offset)<1e-5)))throw Error('Size consumes an adjacent face. Use a smaller value.')
 return polygonBody(body,mesh)
}
export function shellSolid(body:DirectBody,openingFaces:number[],thickness:number):DirectBody {
 if(!Number.isFinite(thickness)||thickness<.01||!openingFaces.length)throw Error('Select at least one opening and a positive wall thickness.')
 const planes=convexPlanes(body),open=new Set(openingFaces);if(open.size>=planes.length||[...open].some(i=>!planes[i]))throw Error('Keep at least one closed face.')
 const p=bodyPoints(body),span=Math.max(...[0,1,2].map(k=>Math.max(...p.map(v=>v[k]))-Math.min(...p.map(v=>v[k]))))*3+thickness
 const inner=planes.map((f,i)=>({...f,offset:f.offset+(open.has(i)?span:-thickness)}))
 // Ensure a real cavity exists within the original solid, before extending it through the openings.
 fromPlanes(planes.map((f,i)=>({...f,offset:f.offset-(open.has(i)?0:thickness)})))
 const cutter=fromPlanes(inner),mesh=booleanPolygonMeshes(body.mesh,cutter,'difference')
 if(!mesh.report.closed||!mesh.indices.length)throw Error('The wall thickness collapses the body.')
 return polygonBody(body,{positions:mesh.positions,indices:mesh.indices})
}
export function splitSolid(body:DirectBody,normal:Vec3,offset:number):[DirectBody,DirectBody] {
 if(!Number.isFinite(offset))throw Error('Enter a finite plane offset.')
 normal=unit3(normal)
 const p=bodyPoints(body),values=p.map(p=>dot3(normal,p))
 if(offset<=Math.min(...values)+1e-6||offset>=Math.max(...values)-1e-6)throw Error('The plane must intersect the body.')
 const u=unit3(cross3(normal,Math.abs(normal[0])<.8?[1,0,0]:[0,1,0])),v=cross3(normal,u),origin=mul(normal,offset)
 const size=Math.max(...p.map(p=>Math.hypot(...sub(p,origin))))*2+1
 const box=extrudePolygonProfile({outer:[[-size,-size],[size,-size],[size,size],[-size,size]]},[0,0,size])
 const cutter={positions:Array.from({length:box.positions.length/3},(_,i)=>worldPoint(box.positions.slice(i*3,i*3+3),{origin,u,v})).flat(),indices:box.indices}
 const a=booleanPolygonMeshes(body.mesh,cutter,'intersection'),b=booleanPolygonMeshes(body.mesh,cutter,'difference')
 if(!a.report.closed||!b.report.closed||!a.indices.length||!b.indices.length)throw Error('Could not create two closed halves.')
 return [polygonBody({...body,name:(body.name+' · +').slice(0,100)},{positions:a.positions,indices:a.indices}),polygonBody({...body,id:body.id+'-split',name:(body.name+' · −').slice(0,100)},{positions:b.positions,indices:b.indices})]
}
function transformBrep(body:DirectBody,apply:(point:number[])=>Vec3):DirectBody {
 const next={...body,mesh:{...body.mesh,positions:bodyPoints(body).map(apply).flat()}}
 if(!body.brep)return next
 const brep=structuredClone(body.brep)
 for(const vertex of brep.vertices)vertex.point=apply(vertex.point)
 for(const edge of brep.edges)edge.curve.controlPoints=edge.curve.controlPoints.map(apply)
 for(const face of brep.faces)face.surface.controlPoints=face.surface.controlPoints.map(row=>row.map(apply))
 return {...next,brep}
}
export function transformBodies(bodies:DirectBody[],delta:Vec3,axis:Vec3,angle:number,scale:number):DirectBody[] {
 if(![...delta,...axis,angle,scale].every(Number.isFinite)||scale<=0)throw Error('Invalid transform.')
 const points=bodies.flatMap(bodyPoints);if(!points.length)return []
 const center=[0,1,2].map(k=>(Math.min(...points.map(p=>p[k]))+Math.max(...points.map(p=>p[k])))/2),n=unit3(axis),a=angle*Math.PI/180,c=Math.cos(a),s=Math.sin(a)
 return bodies.map(b=>transformBrep(b,p=>{const q=mul(sub(p,center),scale),rot=add(add(mul(q,c),mul(cross3(n,q),s)),mul(n,dot3(n,q)*(1-c)));return add(add(rot,center),delta)}))
}

/** Transform the entire selection around one world-space pivot, preserving analytic sketches. */
export function transformSelection(document: import('./directModeling').DirectDocument, ids:string[],delta:Vec3,axis:Vec3,angle:number,scale:number):import('./directModeling').DirectDocument {
 if(![...delta,...axis,angle,scale].every(Number.isFinite)||scale<=0)throw Error('Invalid transform.')
 const next=structuredClone(document),selected=new Set(ids),points=[...next.bodies.filter(b=>selected.has(b.id)).flatMap(bodyPoints),...next.sketches.filter(s=>selected.has(s.id)).flatMap(s=>s.points.map(p=>worldPoint(p,s.plane)))]
 if(!points.length)return next
 const center=[0,1,2].map(k=>(Math.min(...points.map(p=>p[k]))+Math.max(...points.map(p=>p[k])))/2),n=unit3(axis),a=angle*Math.PI/180,c=Math.cos(a),s=Math.sin(a)
 const rotate=(q:number[])=>add(add(mul(q,c),mul(cross3(n,q),s)),mul(n,dot3(n,q)*(1-c)))
 const apply=(p:number[])=>add(add(rotate(mul(sub(p,center),scale)),center),delta)
 for(let i=0;i<next.bodies.length;i++)if(selected.has(next.bodies[i].id))next.bodies[i]=transformBrep(next.bodies[i],apply)
 for(const sketch of next.sketches)if(selected.has(sketch.id)){
  const plane=sketch.plane??{origin:[0,0,0] as Vec3,u:[1,0,0] as Vec3,v:[0,1,0] as Vec3}
  const movedPlane={origin:apply(plane.origin),u:rotate(plane.u),v:rotate(plane.v)},normal=cross3(plane.u,plane.v)
  if(dot3(normal,cross3(movedPlane.u,movedPlane.v))>1-1e-8&&Math.abs(dot3(sub(movedPlane.origin,plane.origin),normal))<1e-7){
    // Keep a coplanar edit in the existing sketch coordinate system so 2D views stay shared.
    const local=(p:number[]):[number,number]=>{const q=sub(apply(worldPoint(p,plane)),plane.origin);return [dot3(q,plane.u),dot3(q,plane.v)]}
    sketch.points=sketch.points.map(local)
    if(sketch.analytic){sketch.analytic.center=local(sketch.analytic.center);sketch.analytic.radius*=scale;sketch.analytic.start+=Math.atan2(dot3(movedPlane.u,plane.v),dot3(movedPlane.u,plane.u))*180/Math.PI}
  }else{
    sketch.plane=movedPlane
    sketch.points=sketch.points.map(p=>[p[0]*scale,p[1]*scale])
    if(sketch.analytic){sketch.analytic.center=sketch.analytic.center.map(v=>v*scale) as [number,number];sketch.analytic.radius*=scale}
  }
 }
 return next
}
