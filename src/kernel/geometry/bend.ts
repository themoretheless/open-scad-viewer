import { callGeometryRust, withCadMesh } from './kernel'
import { deformPolygonMesh, normalizePolygonMesh, transformPolygonMesh, type PolygonMesh } from './polygon'

/** Refine only edges along the bend, using Rust's conforming shared-edge knife. */
export function bendCadMesh(id:number,radius:number,center:number,reverse:boolean,step:number):PolygonMesh {
  if(!Number.isFinite(radius)||radius<=0||!Number.isFinite(center)||!Number.isFinite(step)||step<=0) throw Error('Invalid bend dimensions')
  const direction=reverse?-1:1
  let mesh:PolygonMesh=withCadMesh(id,raw=>({positions:new Float64Array(raw.positions),indices:new Uint32Array(raw.indices)}))
  // Axes for the existing Rust cylindrical bend: X is preserved, Z is the arc axis.
  mesh=transformPolygonMesh(mesh,[[0,0,1,0],[0,direction,0,0],[1,0,0,-center],[0,0,0,1]])
  for(let pass=0;pass<24;pass++) {
    const edges=callGeometryRust<Array<[number,number]>>('mesh_editor',{action:'edges',mesh})
    const ids:number[]=[]
    edges.forEach(([a,b],i)=>{if(Math.abs(mesh.positions[a*3+2]-mesh.positions[b*3+2])>step*(1+1e-9))ids.push(i)})
    if(ids.length===0) {
      const bent=deformPolygonMesh(mesh,{kind:'bend',origin:[0,0,0],radius})
      return transformPolygonMesh(bent,[[0,0,1,center],[0,direction,0,0],[1,0,0,0],[0,0,0,1]])
    }
    mesh=normalizePolygonMesh(callGeometryRust<PolygonMesh>('mesh_editor',{action:'knife',mesh,ids}))
  }
  throw Error('Bend refinement did not converge')
}
