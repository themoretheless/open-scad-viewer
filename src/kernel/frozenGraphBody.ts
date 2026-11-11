import {callNurbsRust} from './geometry/nurbs'
export interface FrozenGraphRestoreOptions {maxBytes?:number;maxWork?:number;maxPairs?:number}
export type FrozenGraphFormat='native-convex-graph-volume/1'|'native-convex-graph-volume/2'
export interface FrozenGraphExpectation {format:FrozenGraphFormat;geometryText:string;maxBytes:number;maxWork:number;maxPairs:number}
export interface FrozenGraphBody {
 representation:'native-exact-graph-body'
 artifact:{format:FrozenGraphFormat;recipe:unknown;geometry:Record<string,unknown>}
 report:{method:'native-frozen-graph-body-restore/1';nativeVolumeCertified:true;globalEmbeddingCertified:true;
 faces:number;edges:number;vertices:number;bodies:1;openEdges:0;pairProducts:number;allPairsReproved:true;
 internalG2Joins:number;curvatureLimitPerMm:10;absoluteCurvatureUpper:number;signedVolumeMm3:[string,string];
 coefficientWork:number;maxWork:number;maxPairs:number;maxBytes:number;radiusLawCertified:false;thicknessCertified:false;applicationAdmitted:false}
}
export function frozenGraphExpectation(artifact:unknown,options:FrozenGraphRestoreOptions={}):FrozenGraphExpectation {
 const a=artifact as FrozenGraphBody['artifact']
 if((a?.format!=='native-convex-graph-volume/1'&&a?.format!=='native-convex-graph-volume/2')||!a.geometry||typeof a.geometry!=='object')throw Error('Invalid frozen graph body artifact')
 return {format:a.format,geometryText:JSON.stringify(a.geometry),maxBytes:options.maxBytes??4*1024*1024,maxWork:options.maxWork??10_000_000,maxPairs:options.maxPairs??10000}
}
/** Exact native artifact; no rounded B-rep conversion or document publication. */
export function restoreFrozenGraphBody(artifact:unknown,options:FrozenGraphRestoreOptions={},checkAbort?:()=>void):FrozenGraphBody {
 checkAbort?.();const expected=frozenGraphExpectation(artifact,options)
 const {geometryText:_,format:__,...limits}=expected
 const result=callNurbsRust<FrozenGraphBody>('brep_frozen_graph_body_restore',{artifact,...limits})
 checkAbort?.()
 if(!validFrozenGraphBody(result,expected))throw Error('Invalid frozen graph body restore receipt')
 return result
}
function sameJson(a:unknown,b:unknown,depth=0):boolean {
 if(a===b)return true
 if(depth>128||!a||!b||typeof a!=='object'||typeof b!=='object'||Array.isArray(a)!==Array.isArray(b))return false
 const ka=Object.keys(a),kb=Object.keys(b)
 return ka.length===kb.length&&ka.every(k=>Object.hasOwn(b,k)&&sameJson((a as any)[k],(b as any)[k],depth+1))
}
export function validFrozenGraphBody(value:unknown,expected:FrozenGraphExpectation):value is FrozenGraphBody {
 try{
 const v=value as FrozenGraphBody,r=v.report,g=v.artifact.geometry as any
 if(v.representation!=='native-exact-graph-body'||v.artifact.format!==expected.format||r.method!=='native-frozen-graph-body-restore/1')return false
 if(r.nativeVolumeCertified!==true||r.globalEmbeddingCertified!==true||r.allPairsReproved!==true||r.radiusLawCertified!==false||r.thicknessCertified!==false||r.applicationAdmitted!==false||g.applicationAdmitted!==false||g.radiusLawCertified!==false)return false
 const count=(x:number)=>Number.isSafeInteger(x)&&x>=0
 if(![r.faces,r.edges,r.vertices,r.pairProducts,r.internalG2Joins,r.coefficientWork,r.maxWork,r.maxPairs,r.maxBytes].every(count)||r.faces<1||r.bodies!==1||r.openEdges!==0)return false
 if(r.maxWork!==expected.maxWork||r.maxPairs!==expected.maxPairs||r.maxBytes!==expected.maxBytes||r.coefficientWork>r.maxWork||r.maxWork>10_000_000||r.maxBytes<1||r.maxBytes>4*1024*1024||r.maxPairs<1||r.maxPairs>1_000_000||r.pairProducts!==r.faces*(r.faces-1)/2||r.pairProducts>r.maxPairs)return false
 if(r.curvatureLimitPerMm!==10||!Number.isFinite(r.absoluteCurvatureUpper)||r.absoluteCurvatureUpper<0||r.absoluteCurvatureUpper>10||r.internalG2Joins>=r.faces)return false
 if(!Array.isArray(r.signedVolumeMm3)||r.signedVolumeMm3.length!==2||r.signedVolumeMm3.some(s=>typeof s!=='string'||s.length>2500||! /^[1-9][0-9]*$/.test(s)))return false
 if(!Array.isArray(g.faces)||g.faces.length!==r.faces||!Array.isArray(g.edges)||g.edges.length!==r.edges||!Array.isArray(g.vertices)||g.vertices.length!==r.vertices||!Array.isArray(g.bodies)||g.bodies.length!==1||!g.identityLinks)return false
 if(!sameJson(r.signedVolumeMm3,g.signedVolume)||!sameJson(g,JSON.parse(expected.geometryText)))return false
 return true
 }catch{return false}
}

export interface FrozenGraphDisplay extends FrozenGraphBody {display:{method:'native-exact-graph-edges/1';segments:number;edges:{edge:number;root:number;points:[number,number,number][]}[];sampleStorageErrorUpperMm:number;polylineDeviationCertified:false}}
export function displayFrozenGraphBody(artifact:unknown,segments=8,options:FrozenGraphRestoreOptions={},checkAbort?:()=>void):FrozenGraphDisplay {
 checkAbort?.();const expected=frozenGraphExpectation(artifact,options),{geometryText:_,format:__,...limits}=expected
 const result=callNurbsRust<FrozenGraphDisplay>('brep_frozen_graph_body_display',{artifact,segments,...limits})
 checkAbort?.()
 if(!validFrozenGraphDisplay(result,expected,segments))throw Error('Invalid frozen graph body display receipt')
 return result
}
export function validFrozenGraphDisplay(value:unknown,expected:FrozenGraphExpectation,segments:number):value is FrozenGraphDisplay {
 try{
 if(!validFrozenGraphBody(value,expected)||!Number.isSafeInteger(segments)||segments<1||segments>64)return false
 const d=(value as FrozenGraphDisplay).display,g=value.artifact.geometry as any
 if(d.method!=='native-exact-graph-edges/1'||d.segments!==segments||d.polylineDeviationCertified!==false||!Number.isFinite(d.sampleStorageErrorUpperMm)||d.sampleStorageErrorUpperMm<0||!Array.isArray(d.edges)||d.edges.length!==value.report.edges||d.edges.length*(segments+1)>32768)return false
 const seen=new Set<number>()
 for(const edge of d.edges){
  if(!Number.isSafeInteger(edge.edge)||edge.edge<0||edge.edge>=g.edges.length||seen.has(edge.edge)||edge.root!==g.edges[edge.edge].root||!Array.isArray(edge.points)||edge.points.length!==segments+1||edge.points.some(p=>!Array.isArray(p)||p.length!==3||!p.every(Number.isFinite)))return false
  seen.add(edge.edge)
 }
 return true
 }catch{return false}
}
