import type {DirectBody} from './directModeling'
import {callGeometryRust} from './geometry/kernel'
import {normalizePolygonMesh,type PolygonBuild} from './geometry/polygon'
/** Sampled shell; spacing is a grid limit, not an exact B-rep tolerance. */
export function sampledShell(body:DirectBody,openings:number[],thickness:number,requestedStep?:number,adaptive=false):DirectBody {
 const mesh=normalizePolygonMesh(callGeometryRust<PolygonBuild>('cad_sampled_shell',{mesh:body.mesh,openings,thickness,step:requestedStep??0,adaptive}))
 return {...body,mesh:{positions:mesh.positions,indices:mesh.indices}}
}
/** Local circular edge cutter/filler for legacy triangle-mesh bodies. */
export function localMeshBevel(body:DirectBody,edgeIndex:number,radius:number,kind:'fillet'|'chamfer',endRadius=radius,target=body):DirectBody {
 const mesh=normalizePolygonMesh(callGeometryRust<PolygonBuild>('cad_local_mesh_blend',{mesh:body.mesh,target:target.mesh,edge:edgeIndex,radius,kind,endRadius}))
 return {...target,mesh:{positions:mesh.positions,indices:mesh.indices}}
}
