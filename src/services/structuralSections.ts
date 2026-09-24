import {callGeometryRust} from './geometry/kernel'
import type {PolygonMesh} from './geometry/polygon'

export interface StructuralSection {
  positionMm:number
  material:boolean
  contours:[number,number][][]
  boundaryContours:[number,number][][]
  sourceTriangles:number[][]
  properties:null|{areaMm2:number;centroidMm:[number,number];iuuMm4:number;ivvMm4:number;iuvMm4:number}
}
export interface BoundaryConnectivity {
  modelKind:'indexed-edge-boundary-components-v1'
  materialConnectivity:'not-established'|'classified-at-tolerance'
  components:{id:number;sourceTriangles:number[];signedVolumeMm3:number;boundsMm:[number[],number[]]}[]
  sharedVertices:{sourceVertex:number;components:number[]}[]
}
export type MaterialAudit =
  | {status:'unresolved';code:string;message:string}
  | {status:'classified';toleranceMm:number;materialRegions:number;
      shells:{id:number;firstTriangle:number;parent:number|null;depth:number;kind:'material'|'cavity'}[]}
export interface StructuralSections {
  modelKind:'finished-mesh-sections-v1'
  sourceMesh:PolygonMesh
  axis:'x'|'y'|'z'
  planeAxes:[number,number]
  volumeMm3:number
  triangleCount:number
  selfIntersections:'not-checked'|'checked-at-tolerance'
  materialAudit:MaterialAudit
  connectivity:BoundaryConnectivity
  sections:StructuralSection[]
}
export function inspectStructuralSections(mesh:PolygonMesh,axis:'x'|'y'|'z',stations:number[]):StructuralSections {
  return callGeometryRust('structural_sections',{mesh,axis,stations})
}
