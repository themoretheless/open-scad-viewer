import {callGeometryRust} from './geometry/kernel'
import {normalizePolygonMesh,type PolygonMesh} from './geometry/polygon'
export function decimateLattice(mesh:PolygonMesh,tolerance:number,target=2600):PolygonMesh{
 return normalizePolygonMesh(callGeometryRust<PolygonMesh>('cad_lattice_decimate',{mesh,tolerance,target}))
}
