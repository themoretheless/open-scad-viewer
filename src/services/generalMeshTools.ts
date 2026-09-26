import {bodyPoints,type DirectBody,type Point2} from './directModeling'
import {solidTopology} from './directSolidTools'
import {callGeometryRust} from './geometry/kernel'
import {booleanPolygonMeshes,extrudePolygonProfile,loftPolygonSections,inspectPolygonMesh,normalizePolygonMesh,type PolygonBuild} from './geometry/polygon'
import {cross3,dot3,unit3,worldPoint,type Vec3} from './directSketchGeometry'
import {directCornerTool} from './directProfileTools'
/** Sampled shell. step is the requested maximum grid spacing, never an exact B-rep tolerance. */
export function sampledShell(body:DirectBody,openings:number[],thickness:number,requestedStep?:number,adaptive=false):DirectBody {
 if(requestedStep!==undefined&&(!Number.isFinite(requestedStep)||requestedStep<0))throw Error('Grid step must be finite and nonnegative.')
 const topology=solidTopology(body.mesh),triangles=[...new Set(openings.flatMap(i=>{if(!topology.faces[i])throw Error('Invalid opening face.');return topology.faces[i].triangles}))]
 const p=bodyPoints(body),span=Math.max(...[0,1,2].map(k=>Math.max(...p.map(v=>v[k]))-Math.min(...p.map(v=>v[k]))))
 const step=requestedStep&&requestedStep>0?requestedStep:adaptive?thickness/4:Math.max(thickness/4,span/59)
 const mesh=normalizePolygonMesh(callGeometryRust<PolygonBuild>(adaptive?'mesh_shell_adaptive':'mesh_shell_sampled',{mesh:body.mesh,openings:triangles,thickness,step}))
 return {...body,mesh:{positions:mesh.positions,indices:mesh.indices}}
}
/** Local circular edge cutter/filler; adjacent faces may belong to a nonconvex body. */
export function localMeshBevel(body:DirectBody,edgeIndex:number,radius:number,kind:'fillet'|'chamfer',endRadius=radius,target=body):DirectBody {
 if(!Number.isFinite(radius)||radius<.01||!Number.isFinite(endRadius)||endRadius<.01)throw Error('Radius must be at least 0.01 mm.')
 const report=inspectPolygonMesh(body.mesh);if(!report.closed||report.signedVolumeMm3<=0)throw Error('Select a closed outward-oriented body.')
 const topology=solidTopology(body.mesh),edge=topology.edges[edgeIndex],p=bodyPoints(body)
 if(!edge)throw Error('Select a manifold edge.')
 const a=p[edge.a],b=p[edge.b],length=Math.hypot(...b.map((v,k)=>v-a[k])),axis=unit3(b.map((v,k)=>v-a[k])),middle=a.map((v,k)=>(v+b[k])/2)
 const adjacent=edge.faces.map(i=>{const face=topology.faces[i];for(const t of face.triangles){const ids=body.mesh.indices.slice(t*3,t*3+3);if(ids.includes(edge.a)&&ids.includes(edge.b))return p[ids.find(j=>j!==edge.a&&j!==edge.b)!]}return face.center})
 const sides=adjacent.map(point=>{const q=point.map((v,k)=>v-middle[k]),along=dot3(q,axis);return unit3(q.map((v,k)=>v-axis[k]*along))})
 const u=sides[0],v=cross3(axis,u),second:[number,number]=[dot3(sides[1],u),dot3(sides[1],v)]
 const theta=Math.acos(Math.max(-1,Math.min(1,second[0])))
 if(theta<.01||Math.PI-theta<.01)throw Error('The selected edge has no usable corner angle.')
 const tangent=kind==='fillet'?Math.max(radius,endRadius)/Math.tan(theta/2):Math.max(radius,endRadius)
 // Prevent a radius extending beyond either incident face's perpendicular extent.
 const widths=edge.faces.map((f,i)=>Math.max(...topology.faces[f].vertices.map(j=>dot3(p[j].map((x,k)=>x-middle[k]),sides[i]))))
 if(tangent>=Math.min(...widths)*.95||tangent>=length/2)throw Error('Radius consumes an adjacent face or endpoint; reduce it.')
 const reach=tangent*2+radius,triangle:Point2[]=[[0,0],[reach,0],[second[0]*reach,second[1]*reach]]
 const arc=kind==='fillet'?directCornerTool({id:'edge',name:'edge',points:triangle,closed:true},0,radius,'fillet').points.slice(0,-2):[[second[0]*radius,second[1]*radius],[radius,0]] as Point2[]
 const profile:Point2[]=[[0,0],...arc],plane={origin:a as Vec3,u,v}
 const cutter=loftPolygonSections([profile.map(p=>worldPoint([...p,0],plane)),profile.map(p=>worldPoint([p[0]*endRadius/radius,p[1]*endRadius/radius,length],plane))])
 const f0=topology.faces[edge.faces[0]],f1=topology.faces[edge.faces[1]],concave=dot3(f0.normal,adjacent[1])>f0.offset+1e-6
 const mesh=booleanPolygonMeshes(target.mesh,cutter,concave?'union':'difference')
 if(!mesh.report.closed||mesh.report.signedVolumeMm3<=0||Math.abs(mesh.report.signedVolumeMm3-inspectPolygonMesh(target.mesh).signedVolumeMm3)<1e-8)throw Error('Local edge blend did not produce a valid changed solid.')
 return {...target,mesh:{positions:mesh.positions,indices:mesh.indices}}
}
