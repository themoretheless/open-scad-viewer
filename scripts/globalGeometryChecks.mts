// Qualification assertions and transport only. Every geometric operation is Rust.
import {callGeometryRust as call} from '../src/services/geometry/kernel'
const requireCheck=(condition:unknown,message:string)=>{if(!condition)throw Error(message)}
export function runGlobalGeometryChecks(fixtures:any,progress=(message:string)=>{}) {
 const cases:any[]=[]
 for(const entry of fixtures.cases){
  const original=JSON.stringify(entry.request)
  let value:any,error:any
  try{value=call('surface_progressive_sweep_profiles',entry.request)}catch(e){error=e}
  requireCheck(JSON.stringify(entry.request)===original,`${entry.id}: input mutation`)
  if(entry.expectedError){
   requireCheck(error?.code===entry.expectedError,`${entry.id}: expected ${entry.expectedError}, received ${error?.code}`)
   cases.push({id:entry.id,status:'typed-refusal',code:error.code})
  }else{
   if(error)throw error
   requireCheck(value.report.accepted&&value.report.continuousBound,`${entry.id}: original transport bound absent`)
   const volume:any=call('brep_sweep_volume_audit',{...fixtures.volumeBudgets,model:entry.model})
   requireCheck(volume.solidGeometryCertified&&volume.regularBoundarySolidCertified,`${entry.id}: joint authored boundary unproven`)
   cases.push({id:entry.id,status:'passed',volume})
  }
  progress(`${cases.length}/${fixtures.cases.length}: ${entry.id}`)
 }
 const intersections=fixtures.intersectionCases.map((request:any)=>{
  const {op,...args}=request,report:any=call(op,args)
  requireCheck(report.coverage==='numerically_resolved'&&!report.permitsTopologyChange&&report.unresolved.length===0,`${op}: invalid scope`)
  let constraints=0
  for(const circle of report.components)for(const [key,lifts] of Object.entries(circle)){
   if(!key.endsWith('Uv')||!Array.isArray(lifts))continue
   for(const lift of lifts as any[]){
    if(!lift.implicitContour)continue
    const c=lift.implicitContour
    requireCheck(lift.arcs.length===0&&c.representation==='implicit-surface-plane-pullback'&&!c.arcComponentsEnumerated&&!c.permitsTopologyChange,`${op}: false rational/topology claim`)
    constraints++
   }
  }
  requireCheck(constraints>=8,`${op}: missing pole-chart source constraints`)
  return {op,components:report.components.length,constraints,coverage:report.coverage,permitsTopologyChange:report.permitsTopologyChange}
 })
 const sphere=call<any>('brep_nurbs_sphere_exact',{radius:2})
 const pole:any=call('brep_sweep_volume_audit',{...fixtures.volumeBudgets,model:sphere})
 requireCheck(pole.regularBoundarySolidCertified&&pole.boundaryRegularity.allFacesRegularModuloPoles&&!pole.boundaryRegularity.allFacesRegular,'pole stratification or volume evidence lost')
 const positive=fixtures.cases.find((c:any)=>!c.expectedError)
 const exhausted:any=call('brep_sweep_volume_audit',{...fixtures.volumeBudgets,model:positive.model,maxPairs:1,maxRegularityCells:0})
 requireCheck(!exhausted.solidGeometryCertified&&!exhausted.regularBoundarySolidCertified,'exhausted pair/regularity budget admitted')
 const passed=cases.filter(c=>c.status==='passed').length,refused=cases.length-passed
 requireCheck(passed===72&&refused===24&&intersections.length===5,'incomplete finite matrix')
 return {schema:1,scope:'Finite authored saddle-cap matrix and read-only pole-sphere queries; no arbitrary-frame, G1 or production approval inferred',passed,refused,cases,intersections,pole,exhausted}
}
