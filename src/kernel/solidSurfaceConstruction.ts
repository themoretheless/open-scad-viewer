import {DirectInputError} from '../i18n/kernelErrors'
import {reverseNurbsCurve} from './nurbsCurve'
import type {DirectDocument} from '../app/directModeling'
import type {SolidNurbsSurface} from './solidNurbs'
import {loftAlignedNurbsCurves,sweepNurbsCurve,coonsNurbsPatch,framedSweepNurbsCurve,type FramedSweepResult,prepareCoonsBoundaryWeights,type CoonsBoundaryPreparation} from './nurbsConstructors'
import {createG2WorldRunout,type G2RunoutReport} from './geometricG2'
import {applySolidSweepBody,type SolidSweepBodyOptions} from './solidSweepBody'

export type SolidSurfaceConstruction = 'nurbs-loft'|'nurbs-sweep'|'nurbs-patch'|'nurbs-runout'
export interface SweepOptions {mode:'translation'|'framed';normal:[number,number,number];sections:number;maxDeviation:number}
/** Build an independent native surface. Source curves and their identities are retained. */
export function constructSolidSurface(document:DirectDocument,ids:string[],kind:SolidSurfaceConstruction,id:string,group?:string,reversed:boolean[]=[],sweep?:SweepOptions,onSweepReport?:(report:FramedSweepResult['report'])=>void):SolidNurbsSurface {
 if(kind==='nurbs-runout')throw Error('RUNOUT_INPUTS: use the worker surface builder')
 if(new Set(ids).size!==ids.length)throw new DirectInputError('DIRECT_CURVES_DISTINCT','Choose distinct curves.',undefined,ids.filter((id,i)=>ids.indexOf(id)!==i))
 if(kind==='nurbs-patch'&&ids.length!==4)throw new DirectInputError('DIRECT_SURFACE_INPUTS','Patch requires four ordered boundaries.',undefined,ids)
 if(kind==='nurbs-sweep'&&ids.length!==2)throw new DirectInputError('DIRECT_SURFACE_INPUTS','Sweep needs a profile and path.',undefined,ids)
 if(kind==='nurbs-loft'&&(ids.length<2||ids.length>32))throw new DirectInputError('DIRECT_SURFACE_INPUTS','Loft requires 2..32 ordered sections.',undefined,ids)
 const curves=ids.map((id,i)=>{const item=document.curves?.find(c=>c.id===id);if(!item)throw new DirectInputError('DIRECT_CURVE_MISSING','Curve missing. Choose inputs again.');return reversed[i]?reverseNurbsCurve(item.curve):item.curve})
 const framed=kind==='nurbs-sweep'&&sweep?.mode==='framed'?framedSweepNurbsCurve(curves[0],curves[1],sweep.normal,sweep.sections,sweep.maxDeviation):null
 if(framed)onSweepReport?.(framed.report)
 if(framed&&!framed.surface)throw new DirectInputError('DIRECT_SWEEP_DEVIATION',`Sweep sampled deviation ${framed.report.sampledControlDeviation} exceeds budget ${framed.report.budget}. Adjust sections or budget.`,'sweepDeviation')
 const surface=kind==='nurbs-patch'?coonsNurbsPatch(curves):kind==='nurbs-sweep'?(framed?.surface??sweepNurbsCurve(curves[0],curves[1])):loftAlignedNurbsCurves(curves)
 return {id,name:kind==='nurbs-patch'?'Coons patch':kind==='nurbs-sweep'?(framed?'Framed sweep':'Translation sweep'):'NURBS loft',surface,segmentsU:24,segmentsV:24,...group?{group}:{}}
}

export interface SolidSurfaceBuildOptions {kind:SolidSurfaceConstruction;ids:string[];id:string;group?:string;reversed:boolean[];sweep:SweepOptions;patchPreparation?:{enabled:boolean;maxError:number};runout?:Parameters<typeof createG2WorldRunout>[3];
 /** Sweep only: close the profile into a solid and add or combine it instead of adding a surface. */
 body?:Pick<SolidSweepBodyOptions,'operation'|'targetId'|'targetIds'|'name'|'tessellation'|'pathTolerance'>}
const SOLID_SURFACE_BUILDS = new Set<string>(['nurbs-loft','nurbs-sweep','nurbs-patch','nurbs-runout'])
export function isSolidSurfaceBuild(kind:unknown):kind is SolidSurfaceConstruction{return typeof kind==='string'&&SOLID_SURFACE_BUILDS.has(kind)}
export function buildSolidSurface(document:DirectDocument,p:SolidSurfaceBuildOptions):{document:DirectDocument|null;error:string;report:FramedSweepResult['report']|null;patchReports?:CoonsBoundaryPreparation['report'][];runoutReport?:G2RunoutReport}{
 let report:FramedSweepResult['report']|null=null
 try{
  if(p.kind==='nurbs-runout'){
   const [sourceId,planeId,curveId]=p.ids
   const source=document.surfaces?.find(s=>s.id===sourceId),plane=document.surfaces?.find(s=>s.id===planeId),curve=document.curves?.find(c=>c.id===curveId)
   if(p.ids.length!==3||new Set(p.ids).size!==3||!source||!plane||!curve||!p.runout)throw Error('RUNOUT_INPUTS')
   if(curve.curve.controlPoints[0]?.length!==3)throw Error('RUNOUT_TARGET: select a spatial boundary curve')
   const result=createG2WorldRunout(source.surface,plane.surface,curve.curve,p.runout)
   if(!result.report.complete||!result.report.withinTolerance)return {document:null,error:result.report.fields.some(f=>f.curvatureDecision==='exceeds')?'RUNOUT_CURVATURE_LIMIT':result.report.seamWithinTolerance&&!result.report.fieldComplete?'RUNOUT_FIELD_UNPROVEN':'RUNOUT_G2_UNPROVEN',report:null,runoutReport:result.report}
   const surfaces=result.patches.map((surface,i)=>({id:i===0?p.id:`${p.id}:${i}`,name:`G2 transition ${i+1}`,surface,segmentsU:24,segmentsV:24,...p.group?{group:p.group}:{}}))
   if(document.surfaces?.length){
    const existingIds=new Set(document.surfaces.map(old=>old.id))
    if(surfaces.some(s=>existingIds.has(s.id)))throw Error('RUNOUT_ID_EXISTS')
   }
   return {document:{...document,surfaces:[...document.surfaces??[],...surfaces]},error:'',report:null,runoutReport:result.report}
  }
  if(p.kind==='nurbs-sweep'&&p.body)return {document:applySolidSweepBody(document,{...p.body,profileId:p.ids[0]??'',pathId:p.ids[1]??'',reversed:p.reversed,id:p.id}),error:'',report:null}
  let source=document
  const patchReports:CoonsBoundaryPreparation['report'][]=[]
  if(p.kind==='nurbs-patch'&&p.patchPreparation?.enabled){
   source={...document,...(document.curves?{curves:document.curves.map(item=>{
    const role=p.ids.indexOf(item.id)
    if(role<0)return item
    const result=prepareCoonsBoundaryWeights(item.curve,p.patchPreparation!.maxError)
    patchReports.push(result.report)
    if(!result.report.accepted)throw Error(`PATCH_BUDGET:${role}`)
    return {...item,curve:result.curve}
   })}:{})}
  }
  const surface=constructSolidSurface(source,p.ids,p.kind,p.id,p.group,p.reversed,p.sweep,value=>{report=value})
  return {document:{...document,surfaces:[...document.surfaces??[],surface]},error:'',report,...patchReports.length?{patchReports}:{}}
 }catch(error){
  // Coded input refusals cross the worker boundary with their field and target ids; the sweep budget keeps its report.
  if(error instanceof DirectInputError&&error.code!=='DIRECT_SWEEP_DEVIATION')throw error
  return {document:null,error:error instanceof Error?error.message:String(error),report}
 }
}
