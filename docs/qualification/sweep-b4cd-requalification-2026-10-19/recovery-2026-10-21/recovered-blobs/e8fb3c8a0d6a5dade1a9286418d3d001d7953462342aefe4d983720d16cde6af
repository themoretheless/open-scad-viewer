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

export interface G2RunoutReport {
 method:'native-original-support-G2-runout/1'
 complete:boolean;withinTolerance:boolean;cells:number;exactWork:number;maxCells:number;maxExactWork:number
 tolerances:[number,number,number]
 solidGeometryCertified:false;radiusLawCertified:false;interiorEmbeddingCertified:false
 supportSeams:{patch:number;source:number;positionValidated:boolean;positionUpper:number|null;inspection:NonNullable<GeometricG2Report['seams'][number]['inspection']>}[]
 internalSeams:NonNullable<GeometricG2Report['seams'][number]['inspection']>[]
}
/** Native open runout construction. Unresolved receipts remain available for diagnostics. */
export function createG2RunoutSurface(source:import('./nurbsSurface').NurbsSurface,plane:import('./nurbsSurface').NurbsSurface,
 targetPcurve:import('./nurbsCurve').NurbsCurve,
 options:{sourceBoundary:'uMin'|'uMax'|'vMin'|'vMax';sourceTrace:{at:number;along:[number,number]};transverseUv:[number,number];
  gains:[number,number];reverseU?:[boolean,boolean];maxPatches?:number;tolerances?:[number,number,number];maxCells?:number;maxExactWork?:number},
 checkAbort?:()=>void):{patches:import('./nurbsSurface').NurbsSurface[];report:G2RunoutReport} {
 checkAbort?.()
 const maxCells=options.maxCells??100000,maxExactWork=options.maxExactWork??1000000
 const result=callNurbsRust<{patches:import('./nurbsSurface').NurbsSurface[];report:G2RunoutReport}>('brep_g2_runout_surface',{
  source,plane,targetPcurve,...options,reverseU:options.reverseU??[false,false],maxPatches:options.maxPatches??256,
  tolerances:options.tolerances??[1e-7,1e-5,1e-4],maxCells,maxExactWork})
 checkAbort?.()
 if(!validG2RunoutResult(result,maxCells,maxExactWork,options.maxPatches??256,options.tolerances??[1e-7,1e-5,1e-4]))throw Error('Native runout receipt or seam coverage is invalid')
 return result
}
export function validG2RunoutResult(value:unknown,maxCells:number,maxExactWork:number,maxPatches:number,tolerances:[number,number,number]=[1e-7,1e-5,1e-4]):value is ReturnType<typeof createG2RunoutSurface> {
 try{
 const result=value as ReturnType<typeof createG2RunoutSurface>
 const r=result?.report
 if(!result||!Array.isArray(result.patches)||!result.patches.length||result.patches.length>maxPatches||!r||!Array.isArray(r.supportSeams)||!Array.isArray(r.internalSeams))return false
 if(typeof r.complete!=='boolean'||typeof r.withinTolerance!=='boolean'||!Array.isArray(r.tolerances)||r.tolerances.length!==3||r.tolerances.some((x,i)=>x!==tolerances[i])
  ||r.supportSeams.length>result.patches.length*2||r.internalSeams.length>result.patches.length-1)return false
 for(const s of result.patches){
  const n=s.controlPoints?.length,m=s.controlPoints?.[0]?.length
  if(!n||!m||n*m>4096||![s.degreeU,s.degreeV].every(Number.isSafeInteger)||s.degreeU<1||s.degreeU>=n||s.degreeV<1||s.degreeV>=m
   ||!Array.isArray(s.knotsU)||!Array.isArray(s.knotsV)||s.knotsU.length!==n+s.degreeU+1||s.knotsV.length!==m+s.degreeV+1
   ||[s.knotsU,s.knotsV].some(k=>k.some((v,i)=>!Number.isFinite(v)||(i>0&&v<k[i-1])))
   ||!Array.isArray(s.weights)||s.weights.length!==n||s.weights.some(row=>!Array.isArray(row)||row.length!==m||row.some(w=>!Number.isFinite(w)||w<=0))
   ||s.controlPoints.some(row=>!Array.isArray(row)||row.length!==m||row.some(p=>!Array.isArray(p)||p.length!==3||p.some(v=>!Number.isFinite(v)))))return false
 }
 if(r?.method!=='native-original-support-G2-runout/1'||r.solidGeometryCertified!==false||r.radiusLawCertified!==false||r.interiorEmbeddingCertified!==false
  ||r.maxCells!==maxCells||r.maxExactWork!==maxExactWork||!Number.isSafeInteger(r.cells)||r.cells<0||r.cells>maxCells
  ||!Number.isSafeInteger(r.exactWork)||r.exactWork<0||r.exactWork>maxExactWork)return false
 if(r.withinTolerance&&(!r.complete||r.supportSeams.length!==result.patches.length*2
  ||result.patches.some((_,i)=>[0,1].some(side=>r.supportSeams.filter(s=>s.patch===i&&s.source===side).length!==1))
  ||r.supportSeams.some(s=>!s.positionValidated||s.positionUpper===null||!Number.isFinite(s.positionUpper)||s.positionUpper>tolerances[0]||s.inspection.decision!=='within-tolerance'||!s.inspection.regularityProven)
  ||r.internalSeams.length!==Math.max(0,result.patches.length-1)||r.internalSeams.some(s=>s.decision!=='within-tolerance')))
  return false
 return true
 }catch{return false}
}
/** World-curve input and physical tangent lengths are interpreted by Rust. */
export function createG2WorldRunout(source:import('./nurbsSurface').NurbsSurface,plane:import('./nurbsSurface').NurbsSurface,
 targetCurve:import('./nurbsCurve').NurbsCurve,options:{sourceBoundary:'uMin'|'uMax'|'vMin'|'vMax';sourcePosition:number;tangentLengthMm:number;flipSide:boolean;reverse:boolean}):ReturnType<typeof createG2RunoutSurface> {
 const result=callNurbsRust<ReturnType<typeof createG2RunoutSurface>>('brep_g2_runout_surface',{source,plane,targetCurve,...options,
  reverseU:[false,options.reverse],maxPatches:256,maxCells:100000,maxExactWork:1000000,tolerances:[1e-7,1e-5,1e-4]})
 if(!validG2RunoutResult(result,100000,1000000,256))throw Error('Native runout receipt or seam coverage is invalid')
 return result
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
