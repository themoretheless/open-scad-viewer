import type {NurbsSurface} from '../kernel/nurbsSurface'
export type ParameterBox3=[[number,number],[number,number],[number,number]]
export interface SurfaceIsoTargetReport {
 schema:'nurbs-surface-iso-target-window/1'
 status:'present'|'absent-in-window'|'unresolved'
 axis:0|1
 parameter:number
 sourceIsoParameterCoordinates:'normalized-original-transverse-domain'
 parameterCoordinates:['source-natural-t','target-natural-u','target-natural-v']
 window:ParameterBox3
 maxCells:number
 cells:number
 roots:{method:'original-curve-surface-krawczyk';spans:[number,number,number];existenceDomain:ParameterBox3;parameterBox:ParameterBox3;positionBounds:ParameterBox3;crossingSign:1|-1}[]
 presenceProven:boolean
 windowAbsenceProven:boolean
 windowCoverageComplete:boolean
 unresolved:ParameterBox3[]
 allSurfaceBranchesEnumerated:false
 trimAdmission:false
 bodyAdmission:false
}
const interval=(x:unknown):x is [number,number]=>Array.isArray(x)&&x.length===2&&[x[0],x[1]].every(v=>typeof v==='number'&&Number.isFinite(v))&&x[0]<=x[1]
const box=(x:unknown):x is ParameterBox3=>Array.isArray(x)&&x.length===3&&[0,1,2].every(i=>interval(x[i]))
const inside=(a:ParameterBox3,b:ParameterBox3)=>a.every((r,i)=>r[0]>=b[i]![0]&&r[1]<=b[i]![1])
/** Validate evidence and its ownership domain before a client consumes roots. */
export function isSurfaceIsoTargetReport(value:unknown):value is SurfaceIsoTargetReport {
 if(!value||typeof value!=='object')return false
 const r=value as SurfaceIsoTargetReport
 if(r.schema!=='nurbs-surface-iso-target-window/1'||!['present','absent-in-window','unresolved'].includes(r.status)
  ||![0,1].includes(r.axis)||typeof r.parameter!=='number'||!Number.isFinite(r.parameter)||r.parameter<=0||r.parameter>=1
  ||r.sourceIsoParameterCoordinates!=='normalized-original-transverse-domain'
  ||!Array.isArray(r.parameterCoordinates)||r.parameterCoordinates.length!==3||[0,1,2].some(i=>r.parameterCoordinates[i]!==['source-natural-t','target-natural-u','target-natural-v'][i])
  ||!box(r.window)||r.window.some(d=>d[0]>=d[1])||!Number.isInteger(r.maxCells)||r.maxCells<1||r.maxCells>100000
  ||!Number.isInteger(r.cells)||r.cells<1||r.cells>r.maxCells||!Array.isArray(r.roots)||r.roots.length>r.cells
  ||!Array.isArray(r.unresolved)||r.unresolved.length>r.maxCells+1024||typeof r.windowCoverageComplete!=='boolean'
  ||r.presenceProven!==(r.roots.length>0)||r.windowAbsenceProven!==(r.windowCoverageComplete&&r.roots.length===0)
  ||r.windowCoverageComplete!==(r.unresolved.length===0)||r.allSurfaceBranchesEnumerated!==false||r.trimAdmission!==false||r.bodyAdmission!==false)return false
 if(r.status!==(r.presenceProven?'present':r.windowAbsenceProven?'absent-in-window':'unresolved'))return false
 if(![...r.unresolved].every(d=>box(d)&&inside(d,r.window)))return false
 for(const root of r.roots){
  if(!root||root.method!=='original-curve-surface-krawczyk'||!Array.isArray(root.spans)||root.spans.length!==3||![0,1,2].every(k=>Number.isInteger(root.spans[k])&&root.spans[k]!>=0)
   ||![-1,1].includes(root.crossingSign)||!box(root.positionBounds)||!box(root.existenceDomain)||root.existenceDomain.some(d=>d[0]>=d[1])
   ||!inside(root.existenceDomain,r.window)||!box(root.parameterBox)||!inside(root.parameterBox,root.existenceDomain))return false
 }
 return true
}

export interface SurfaceIsoTargetQuery {
 source:NurbsSurface;axis:0|1;parameter:number;target:NurbsSurface;window:ParameterBox3;maxCells:number
}
export interface SurfaceIsoTargetRequest extends SurfaceIsoTargetQuery {type:'surface-iso-target-window/1';id:string;module:WebAssembly.Module}
