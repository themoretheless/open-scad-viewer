import {callGeometryRust} from './geometry/kernel'
import type {PolygonMesh} from './geometry/polygon'
export function decimateLattice(mesh:PolygonMesh,tolerance:number,target=2600):PolygonMesh{
 return callGeometryRust('cad_lattice_decimate',{mesh,tolerance,target})
}
