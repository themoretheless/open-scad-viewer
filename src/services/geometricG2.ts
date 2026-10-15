import type {NurbsBrep} from './geometry/brep'
import {callNurbsRust} from './geometry/nurbs'
export interface GeometricG2Report {
 method:'owned-boundary-geometric-G2-intervals'
 complete:boolean;withinTolerance:boolean
 positionTolerance:number;normalTolerance:number;curvatureTolerance:number
 maxCells:number;cells:number;edgeIds:number[]
 exactIdentityClaim:false;solidGeometryCertified:false
 corners?:{vertex:number;edges:number[];faces:number[];omittedEdges:number[];closedLink:boolean;allIncidentSeamsWithinTolerance:boolean;solidGeometryCertified:false}[]
 seams:{edge:number;faces:number[];inspection:null|{
  positionBounds:[number,number];regularityProven:boolean;cells:number
  decision:'within-tolerance'|'exceeds-tolerance'|'unresolved'
  orientation:'same'|'opposite'|null
  branches:{normalLower:number;normalUpper:number|null;curvatureLower:number;curvatureUpper:number|null}[]
 }}[]
}
/** Scope, topology extraction, interval proof and budget accounting are native. */
export function inspectBrepGeometricG2(model:NurbsBrep,edges:number[],options:{positionTolerance:number;normalTolerance:number;curvatureTolerance:number;maxCells?:number},checkAbort?:()=>void):GeometricG2Report {
 checkAbort?.()
 const report=callNurbsRust<GeometricG2Report>('brep_geometric_g2_audit',{model,edges,...options,maxCells:options.maxCells??128})
 checkAbort?.()
 return report
}

export interface G2AnnularTubeReport {
 transitionSizes?:[number,number,number,number]
 assembly?:{method:'exact-owned-patch-network/1';correspondenceWork:number;maxWork:number;directlyOwnedUses:number;provedUses:number}
 method:'native-g2-annular-cylinder/1';fullBoundaryG2Certified:true;solidGeometryCertified:true
 profileSeams:number;stationSeams:number;exactWork:number;maxWork:number;radialWallThicknessLower:number;angularRadiusBounds:[number,number]
 allFacesInjective:true;allPairsClassified:true;totalPairs:number;coveredPairs:number;nextPair:null
}
/** Native construction qualifies all saved seams and fresh material before return. */
export function createG2AnnularTube(outerRadius:number,innerRadius:number,height:number,transitionSize:number,transitionSizes?:[number,number,number,number]):{model:NurbsBrep;report:G2AnnularTubeReport} {
 const result=callNurbsRust<{model:NurbsBrep;report:G2AnnularTubeReport}>('brep_g2_annular_cylinder',{outerRadius,innerRadius,height,transitionSize,...(transitionSizes?{transitionSizes}:{})})
 if(result.report?.method!=='native-g2-annular-cylinder/1'||result.report.fullBoundaryG2Certified!==true||result.report.solidGeometryCertified!==true||result.report.allFacesInjective!==true||result.report.allPairsClassified!==true||result.report.nextPair!==null||result.report.coveredPairs!==result.report.totalPairs)throw Error('Native G2 annular body qualification is incomplete')
 if(transitionSizes&&(!result.report.transitionSizes||result.report.transitionSizes.some((x,i)=>x!==transitionSizes[i])||result.report.assembly?.method!=='exact-owned-patch-network/1'))throw Error('Native independent G2 transitions were not preserved')
 return result
}

export interface G2SurfaceBridgeReport {
 method:'native-rational-surface-G2-bridge/1'
 complete:boolean;withinTolerance:boolean;cells:number;maxCells:number
 tolerances:[number,number,number];exactIdentityClaim:false;solidGeometryCertified:false
 supportSeams:{patch:number;source:number;sourceRange:[number,number];inspection:NonNullable<GeometricG2Report['seams'][number]['inspection']>}[]
 internalSeams:{patches:[number,number];inspection:NonNullable<GeometricG2Report['seams'][number]['inspection']>}[]
}
/** Open transition surfaces. The returned receipt never authorizes a solid. */
export function createG2SurfaceBridge(a:import('./nurbsSurface').NurbsSurface,b:import('./nurbsSurface').NurbsSurface,options:{boundaries:['uMin'|'uMax'|'vMin'|'vMax','uMin'|'uMax'|'vMin'|'vMax'];gains:[number,number];reverseU?:[boolean,boolean];maxPatches?:number;tolerances?:[number,number,number];maxCells?:number},checkAbort?:()=>void):{patches:import('./nurbsSurface').NurbsSurface[];report:G2SurfaceBridgeReport} {
 checkAbort?.()
 const result=callNurbsRust<{patches:import('./nurbsSurface').NurbsSurface[];report:G2SurfaceBridgeReport}>('brep_g2_surface_bridge',{a,b,...options,reverseU:options.reverseU??[false,false],maxPatches:options.maxPatches??256,tolerances:options.tolerances??[1e-7,1e-7,1e-5],maxCells:options.maxCells??256})
 checkAbort?.()
 return result
}

export interface ClosedNurbsPatchNetworkReport {
 method:'native-closed-rational-patch-network/1'
 solidGeometryCertified:true;fullBoundaryG2Certified:boolean;requireFullG2:boolean
 faces:number;edges:number;correspondenceWork:number;maxCorrespondenceWork:number
 directlyOwnedUses:number;provedUses:number;exactWork:number;maxSmoothnessWork:number
 allFacesInjective:true;allPairsClassified:true;totalPairs:number;coveredPairs:number;nextPair:null
}
/** Sew full retained support, blend and end patches. Native admission is atomic. */
export function assembleClosedNurbsPatches(surfaces:import('./nurbsSurface').NurbsSurface[],reversed:boolean[],options:{tolerance?:number;requireFullG2?:boolean;sources?:NurbsBrep[];projectionHints?:(null|[[number,number,number],[number,number,number]])[];maxCorrespondenceWork?:number;maxSmoothnessWork?:number}={},checkAbort?:()=>void):{model:NurbsBrep;report:ClosedNurbsPatchNetworkReport} {
 checkAbort?.()
 const result=callNurbsRust<{model:NurbsBrep;report:ClosedNurbsPatchNetworkReport}>('brep_closed_patch_network',{surfaces,reversed,...options,tolerance:options.tolerance??1e-8,requireFullG2:options.requireFullG2??true})
 checkAbort?.()
 const r=result.report
 if(r?.method!=='native-closed-rational-patch-network/1'||r.solidGeometryCertified!==true||r.allFacesInjective!==true||r.allPairsClassified!==true||r.nextPair!==null||r.coveredPairs!==r.totalPairs||((options.requireFullG2??true)&&r.fullBoundaryG2Certified!==true))throw Error('Native closed NURBS patch network admission is incomplete')
 return result
}
