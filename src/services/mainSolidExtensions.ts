import {sampledShell,localMeshBevel} from './generalMeshTools'
import {bodyPoints,type DirectBody,type Point2} from './directModeling'
import {solidTopology,facePlane,shellSolid,bevelSolidEdge} from './directSolidTools'
import {polygonBoundaryLoops,extrudePolygonProfile,booleanPolygonMeshes,inspectPolygonMesh} from './geometry/polygon'
import {dot3,unit3,worldPoint,offsetSketch,type Vec3} from './directSketchGeometry'
import {directCornerTool} from './directProfileTools'
/** Recognize an actual straight prism before using a 2D offset/corner construction. */
function prism(body:DirectBody,direction:Vec3){
 const n=unit3(direction),p=bodyPoints(body),topology=solidTopology(body.mesh),levels=p.map(q=>dot3(q,n)),min=Math.min(...levels),max=Math.max(...levels)
 if(max-min<1e-7||levels.some(v=>Math.abs(v-min)>1e-5&&Math.abs(v-max)>1e-5))throw Error('This extension requires a straight prism; general curved shells are not supported.')
 const bottom=topology.faces.findIndex(f=>dot3(f.normal,n)<-1+1e-6),top=topology.faces.findIndex(f=>dot3(f.normal,n)>1-1e-6)
 if(bottom<0||top<0)throw Error('The prism must have two planar end caps.')
 const f=topology.faces[top],plane=facePlane(body,f);plane.origin=plane.origin.map((v,k)=>v-n[k]*(max-min)) as Vec3
 const loops=polygonBoundaryLoops({positions:body.mesh.positions,indices:topology.faces[bottom].triangles.flatMap(t=>body.mesh.indices.slice(t*3,t*3+3))})
 if(loops.length!==1)throw Error('Prisms with holes require a topology-aware offset.')
 const local=(q:number[]):Point2=>{const v=q.map((x,k)=>x-plane.origin[k]);return [dot3(v,plane.u),dot3(v,plane.v)]}
 const points=loops[0].map(i=>local(p[i]))
 if(points.length>1&&Math.hypot(points[0][0]-points.at(-1)![0],points[0][1]-points.at(-1)![1])<1e-7)points.pop()
 const topPoints=topology.faces[top].vertices.map(i=>local(p[i]))
 if(points.some(a=>!topPoints.some(b=>Math.hypot(a[0]-b[0],a[1]-b[1])<1e-5)))throw Error('End profiles differ; this is not a straight prism.')
 return {points,plane,height:max-min,top,bottom,local}
}
function extrude(points:Point2[],plane:ReturnType<typeof prism>['plane'],height:number,base=0){const mesh=extrudePolygonProfile({outer:points},[0,0,height]);return {positions:Array.from({length:mesh.positions.length/3},(_,i)=>worldPoint([mesh.positions[i*3],mesh.positions[i*3+1],mesh.positions[i*3+2]+base],plane)).flat(),indices:mesh.indices}}
function specializedShell(body:DirectBody,openings:number[],thickness:number):DirectBody{
 try{return shellSolid(body,openings,thickness)}catch(original){
  if(!Number.isFinite(thickness)||thickness<.01||!openings.length)throw original
  const faces=solidTopology(body.mesh).faces,first=faces[openings[0]];if(!first)throw original
  let p:ReturnType<typeof prism>
  try{p=prism(body,first.normal)}catch{return sphereShell(body,openings,thickness)}
  if(openings.some(i=>i!==p.top&&i!==p.bottom))throw Error('For a nonconvex/round prism, open one or both end caps.')
  const inner=offsetSketch({id:'inner',name:'inner',points:p.points,closed:true},-thickness),base=openings.includes(p.bottom)?-thickness:thickness,end=openings.includes(p.top)?p.height+thickness:p.height-thickness
  if(end<=base)throw Error('Wall thickness collapses the cavity.')
  const cavity=extrude(inner.points,p.plane,end-base,base),mesh=booleanPolygonMeshes(body.mesh,cavity,'difference')
  if(!mesh.report.closed||!mesh.indices.length)throw Error('Shell produced an invalid solid.')
  return {...body,mesh:{positions:mesh.positions,indices:mesh.indices}}
 }
}
function specializedBevel(body:DirectBody,edgeIndex:number,size:number,kind:'fillet'|'chamfer'):DirectBody{
 try{return bevelSolidEdge(body,edgeIndex,size,kind)}catch(original){
  const t=solidTopology(body.mesh),e=t.edges[edgeIndex],points=bodyPoints(body);if(!e||!Number.isFinite(size)||size<.01)throw original
  const p=prism(body,unit3(points[e.b].map((v,k)=>v-points[e.a][k]))),q=p.local(points[e.a]),vertex=p.points.findIndex(v=>Math.hypot(v[0]-q[0],v[1]-q[1])<1e-5)
  if(vertex<0)throw original
  let profile=p.points
  if(kind==='fillet')profile=directCornerTool({id:'profile',name:'profile',points:profile,closed:true},vertex,size,'fillet').points
  else {const a=profile[(vertex+profile.length-1)%profile.length],b=profile[vertex],c=profile[(vertex+1)%profile.length],la=Math.hypot(a[0]-b[0],a[1]-b[1]),lc=Math.hypot(c[0]-b[0],c[1]-b[1]);if(size>=Math.min(la,lc)/2)throw Error('Chamfer consumes an adjacent edge.');profile=[...profile.slice(0,vertex),[b[0]+(a[0]-b[0])*size/la,b[1]+(a[1]-b[1])*size/la],[b[0]+(c[0]-b[0])*size/lc,b[1]+(c[1]-b[1])*size/lc],...profile.slice(vertex+1)]}
  const mesh=extrude(profile,p.plane,p.height);if(!inspectPolygonMesh(mesh).closed)throw Error('Invalid beveled prism.')
  return {...body,mesh}
 }
}

/** Concentric inner skin for a tessellated sphere; bridge only the requested openings. */
function sphereShell(body:DirectBody,openings:number[],thickness:number):DirectBody{
 const points=bodyPoints(body),center=[0,1,2].map(k=>(Math.min(...points.map(p=>p[k]))+Math.max(...points.map(p=>p[k])))/2),radii=points.map(p=>Math.hypot(...p.map((v,k)=>v-center[k]))),radius=radii.reduce((a,b)=>a+b,0)/radii.length
 if(radii.some(r=>Math.abs(r-radius)>Math.max(1e-5,radius*1e-5)))throw Error('Shell supports convex planar solids, straight prisms and tessellated spheres; this surface needs a general offset kernel.')
 if(thickness>=radius)throw Error('Wall thickness collapses the sphere.')
 const topology=solidTopology(body.mesh),removed=new Set(openings.flatMap(i=>topology.faces[i]?.triangles??[])),count=points.length
 if(!removed.size||removed.size===body.mesh.indices.length/3)throw Error('Keep at least one closed face.')
 const positions=[...body.mesh.positions,...points.map(p=>p.map((v,k)=>center[k]+(v-center[k])*(radius-thickness)/radius)).flat()],indices:number[]=[],boundary=new Map<string,[number,number]>()
 for(let t=0;t<body.mesh.indices.length/3;t++){
  if(removed.has(t))continue
  const [a,b,c]=body.mesh.indices.slice(t*3,t*3+3);indices.push(a,b,c,c+count,b+count,a+count)
  for(const [u,v] of [[a,b],[b,c],[c,a]]){const key=[u,v].sort((a,b)=>a-b).join(',');if(boundary.has(key))boundary.delete(key);else boundary.set(key,[u,v])}
 }
 for(const [a,b] of boundary.values())indices.push(b,a,a+count,b,a+count,b+count)
 const mesh={positions,indices},report=inspectPolygonMesh(mesh)
 if(!report.closed||report.signedVolumeMm3<=0)throw Error('Sphere shell produced invalid topology.')
 return {...body,mesh}
}

export function extendedShell(body:DirectBody,openings:number[],thickness:number,step?:number,adaptive=false):DirectBody{
 if(body.brep){
  throw new Error('Mesh shell cannot be claimed as analytic-shell for B-rep bodies; refuse sampled/mesh fallback')
 }
 try{return specializedShell(body,openings,thickness)}catch{return sampledShell(body,openings,thickness,step,adaptive)}
}
export function extendedBevel(body:DirectBody,edgeIndex:number,size:number,kind:'fillet'|'chamfer'):DirectBody{
 if(body.brep){
  throw new Error(`Mesh ${kind} cannot be claimed as analytic-${kind} for B-rep bodies; refuse localMeshBevel fallback`)
 }
 try{return specializedBevel(body,edgeIndex,size,kind)}catch{return localMeshBevel(body,edgeIndex,size,kind)}
}
