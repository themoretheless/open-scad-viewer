import type {DirectDocument} from './directModeling'
import type {MainSolidWorkerClient} from './mainSolidWorkerClient'
export async function previewSolidCurveOffset(client:MainSolidWorkerClient,document:DirectDocument,p:{id:string;createdId:string;offsetJoin:string;distance:number;maxError:number;locale?:string}){
 try {
 const options={id:p.id,createdId:p.createdId,distance:p.distance,toleranceMm:p.maxError,maxCells:4096,maxPairs:1000000}
 if(p.offsetJoin.startsWith('trim-')){
  const result=await client.run({kind:'trimmedCurveOffset',document,options:{...options,maxWitnessChecks:1000000,intersectionToleranceMm:p.maxError,fillRule:p.offsetJoin==='trim-evenodd'?'evenodd':'nonzero'}})
  return {document:result.document,report:null,trimmedReport:result.report}
 }
 const result=await client.run({kind:'curveOffset',document,options:{...options,join:p.offsetJoin==='bevel'?'bevel':undefined}})
 return {document:result.document,report:result.report,trimmedReport:null}
}
 catch(error){
  if(error && typeof error==='object' && 'code' in error && typeof error.code==='string' && error.code.startsWith('CAD_'))throw error
  const {curveOffsetErrorMessage}=await import('./curveOffsetErrorMessage')
  throw new Error(curveOffsetErrorMessage(error,p.locale??'en',document.curves?.find(curve=>curve.id===p.id)?.name),{cause:error})
 }
}
