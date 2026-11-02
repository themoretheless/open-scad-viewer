import {callNurbsRust} from './geometry/nurbs'
import type {NurbsSurface} from './nurbsSurface'
import type {NurbsCurve} from './nurbsCurve'
export interface SourceGraphPreviewOptions {radiusMm:number;radiusToleranceMm:number;maxWork?:number;maxCells?:number}
export interface SourceGraphPreview {
 patch:NurbsSurface
 report:{method:'native-source-graph-convex-G2-preview/1';representation:'rounded-display';sourceContactJets:[true,true];previewStorageErrorMm:number;
 curvatureLimitPerMm:10;curvatureDecision:'within'|'exceeds'|'unresolved';absoluteCurvatureUpper:number|null;
 radiusDomain:[[0,1],[0,1]];radiusDecision:'within'|'exceeds'|'unresolved';radiusMm:number;radiusToleranceMm:number;
 radiusWitnessUv:[[number,number],[number,number]]|null;radiusWitnessMm:[number,number]|null;
 coefficientWork:number;maxWork:number;solidGeometryCertified:false;interiorEmbeddingCertified:false;applicationAdmitted:false}
}
/** Display-only candidate. This receipt never authorizes appending a solid. */
export function createSourceGraphG2Preview(sources:[NurbsSurface,NurbsSurface],width:NurbsCurve,options:SourceGraphPreviewOptions,checkAbort?:()=>void):SourceGraphPreview {
 checkAbort?.()
 const maxWork=options.maxWork??1_000_000,maxCells=options.maxCells??1000
 const result=callNurbsRust<SourceGraphPreview>('brep_source_graph_g2_preview',{sources,width,...options,maxWork,maxCells})
 checkAbort?.()
 if(!validSourceGraphG2Preview(result,{...options,maxWork}))throw Error('Invalid source graph G2 preview receipt')
 return result
}
export function validSourceGraphG2Preview(value:unknown,expected:SourceGraphPreviewOptions&{maxWork:number}):value is SourceGraphPreview {
 try {
 const {patch:p,report:r}=value as SourceGraphPreview
 if(r.method!=='native-source-graph-convex-G2-preview/1'||r.representation!=='rounded-display'||r.solidGeometryCertified!==false||r.interiorEmbeddingCertified!==false||r.applicationAdmitted!==false)return false
 if(r.sourceContactJets?.length!==2||r.sourceContactJets.some(x=>x!==true)||r.curvatureLimitPerMm!==10||r.maxWork!==expected.maxWork||!Number.isSafeInteger(r.coefficientWork)||r.coefficientWork<0||r.coefficientWork>r.maxWork)return false
 if(!Number.isFinite(r.previewStorageErrorMm)||r.previewStorageErrorMm<0||!Number.isFinite(r.radiusMm)||!Number.isFinite(r.radiusToleranceMm)||r.radiusMm!==expected.radiusMm||r.radiusToleranceMm!==expected.radiusToleranceMm||!(r.radiusMm>0)||!(r.radiusToleranceMm>=0))return false
 if(!['within','exceeds','unresolved'].includes(r.curvatureDecision)||!['within','exceeds','unresolved'].includes(r.radiusDecision)||JSON.stringify(r.radiusDomain)!=='[[0,1],[0,1]]')return false
 if(r.absoluteCurvatureUpper!==null&&(!Number.isFinite(r.absoluteCurvatureUpper)||r.absoluteCurvatureUpper<0))return false
 if(r.curvatureDecision==='within'&&(r.absoluteCurvatureUpper===null||r.absoluteCurvatureUpper>10))return false
 const interval=(x:unknown):x is [number,number]=>Array.isArray(x)&&x.length===2&&x.every(Number.isFinite)&&x[0]<=x[1]
 if(r.radiusWitnessMm!==null&&(!interval(r.radiusWitnessMm)||r.radiusWitnessMm[0]<0))return false
 if(r.radiusWitnessUv!==null&&(!Array.isArray(r.radiusWitnessUv)||r.radiusWitnessUv.length!==2||r.radiusWitnessUv.some(x=>!interval(x)||x[0]<0||x[1]>1)))return false
 if(r.radiusDecision==='exceeds'&&(r.radiusWitnessMm===null||r.radiusWitnessUv===null))return false
 const n=p.controlPoints?.length,m=p.controlPoints?.[0]?.length
 if(!n||!m||n>4096||m>4096||!Number.isSafeInteger(p.degreeU)||!Number.isSafeInteger(p.degreeV)||p.degreeU<1||p.degreeV!==6||p.degreeU>=n||p.degreeV>=m)return false
 for(const [knots,count,degree] of [[p.knotsU,n,p.degreeU],[p.knotsV,m,p.degreeV]] as const)if(!Array.isArray(knots)||knots.length!==count+degree+1||knots.some((v,i)=>!Number.isFinite(v)||(i>0&&v<knots[i-1])))return false
 if(p.controlPoints.some(row=>!Array.isArray(row)||row.length!==m||row.some(x=>!Array.isArray(x)||x.length!==3||!x.every(Number.isFinite)))||p.weights?.length!==n||p.weights.some(row=>!Array.isArray(row)||row.length!==m||row.some(x=>!Number.isFinite(x)||x<=0)))return false
 return true
 }catch{return false}
}
