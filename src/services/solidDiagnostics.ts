import {inspectPolygonMesh,polygonBoundaryLoops,type PolygonMesh} from './geometry/polygon'
import {callGeometryRust} from './geometry/kernel'
export interface SolidPlaneSection {normal:number[];offsetMm:number;candidateTriangles:number;collapsedSegmentTriangles:number[];contours:{points:number[][];sourceTriangles:number[]}[]}
export interface SolidDiagnosticLocations {boundaryEdges:number[][];nonManifoldEdges:number[][];orientationEdges:number[][];degenerateTriangles:number[]}
/** Read-only diagnostics of the displayed mesh; they do not certify its source B-rep. */
export function inspectSolidDisplay(mesh:PolygonMesh,offset:number,normal:number[]=[0,0,1]) {
 const report=inspectPolygonMesh(mesh)
 const locations=callGeometryRust<SolidDiagnosticLocations>('cad_mesh_diagnostic_locations',{mesh})
 const point=(i:number)=>Array.from(mesh.positions.slice(i*3,i*3+3))
 const defectLines=[
  ...locations.boundaryEdges.map(edge=>({kind:'boundary',points:edge.map(point)})),
  ...locations.nonManifoldEdges.map(edge=>({kind:'non-manifold',points:edge.map(point)})),
  ...locations.orientationEdges.map(edge=>({kind:'orientation',points:edge.map(point)})),
  ...locations.degenerateTriangles.map(i=>{const [a,b,c]=Array.from(mesh.indices.slice(i*3,i*3+3));return {kind:'degenerate',points:[a,b,c,a].map(point)}}),
 ]
 let boundaries:number[][][]=[],boundaryError='',sectionError=''
 let section:SolidPlaneSection={normal,offsetMm:offset,candidateTriangles:0,contours:[],collapsedSegmentTriangles:[]}
 try{boundaries=report.boundaryEdges?polygonBoundaryLoops(mesh).map(loop=>loop.map(i=>Array.from(mesh.positions.slice(i*3,i*3+3)))):[]}catch(e){boundaryError=e instanceof Error?e.message:String(e)}
 try{section=callGeometryRust<SolidPlaneSection>('cad_plane_section',{mesh,normal,offset})}catch(e){sectionError=e instanceof Error?e.message:String(e)}
 return {report,locations,defectLines,boundaries,section,boundaryError,sectionError}
}

export interface SolidMeshContact {triangles:number[];point:number[];sharedVertices:number;toleranceMm:number}
export interface SolidIntersectionOptions {maxWork?:number;maxContacts?:number}
export interface SolidIntersectionReport {contacts:SolidMeshContact[];complete:boolean;stopReason:'work-limit'|'contact-limit'|null;work:number;maxWork:number;maxContacts:number;relativeTolerance:number;scope:'display-mesh-all-contacts'}
/** Enumerates disallowed mesh pairs; complete=false never certifies their absence. */
export function inspectSolidIntersections(mesh:PolygonMesh,options:SolidIntersectionOptions={}) {
 const result=callGeometryRust<SolidIntersectionReport>('cad_mesh_intersections',{mesh,relativeTolerance:1e-9,maxWork:options.maxWork??200_000,maxContacts:options.maxContacts??10_000})
 const triangleIds=[...new Set(result.contacts.flatMap(c=>c.triangles))].sort((a,b)=>a-b)
 return {...result,contact:result.contacts[0]??null,triangleIds,lines:triangleIds.map(i=>{
  const vertices=Array.from(mesh.indices.slice(i*3,i*3+3));vertices.push(vertices[0])
  return vertices.map(v=>Array.from(mesh.positions.slice(v*3,v*3+3)))
 })}
}
