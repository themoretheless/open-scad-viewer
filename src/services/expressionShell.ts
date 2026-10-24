import {validAlignedChordRequest,validAlignedChordReport,type AlignedChordRequest,type AlignedChordReport} from './expressionAlignedChords'
import {validNeighborTangencyRequest,validNeighborTangencyReport,type NeighborTangencyRequest,type NeighborTangencyReport} from './expressionNeighborTangency'
import {validSupportTangencyRequest,validSupportTangencyReport,type SupportTangencyRequest,type SupportTangencyReport} from './expressionSupportTangency'
import {validExpressionRadiusRequest,validExpressionRadiusReport,type ExpressionRadiusRequest,type ExpressionRadiusReport} from './expressionRadiusCheck'
import {validBoundaryMesh,validBoundaryMeshLimits,type BoundaryMeshLimits,type BoundaryMeshReport} from './expressionBoundaryMesh'
import {callGeometryRust} from './geometry/kernel'
import type {SourceBodyOptions} from './sourceBody'
import {validExpressionDisplay,validExpressionDisplayLimits,type ExpressionDisplayLimits,type ExpressionDisplayReport} from './expressionDisplay'
import {validExpressionMesh,validExpressionMeshLimits,type ExpressionMeshLimits,type ExpressionMeshReport} from './expressionMesh'
export interface ExpressionShellOptions {
 alignedChordCheck?:AlignedChordRequest
 neighborTangencyCheck?:NeighborTangencyRequest
 supportTangencyCheck?:SupportTangencyRequest
 radiusCheck?:ExpressionRadiusRequest
 boundaryMeshLimits?:BoundaryMeshLimits
 displayLimits?:ExpressionDisplayLimits
 meshLimits?:ExpressionMeshLimits
 proposalDefinition:unknown;boundaryDefinition:unknown
 sourceLimits:SourceBodyOptions['limits']
 profileLimits:{controls:number;fragments:number;positionTolerance:number}
 shellLimits:{boundary:{seam:{cutSpans:number;mappingCells:number;maxSineSquared:number;angularCells:number;seamSpans:number};controls:number;sideCells:number;sideSpans:number};projection:number[][];cutSpansPerSection:number;mappingCellsPerSection:number;fragmentsPerSection:number;controls:number;joinCells:number;joinSpans:number;exteriorDrivers:number;collapsedEnds:number[];subdivisions:number;quotientCells:number;retainedCells:number;retainedSpans:number;transitionCells:number;transitionSpans:number;exactWorkPerRail:number;driversPerRail:number;fragmentsPerRail:number;outerExactWork:number;outerDrivers:number;shellUses:number}
}
export interface ExpressionShellResult {
 version:1;method:'exact-expression-shell-replay';request:ExpressionShellOptions
 proposalDefinition:unknown;boundaryDefinition:unknown
 incidenceQualified:boolean;embeddingQualified:false;materialQualified:false;reason:string
 shell:null|{faces:{id:number;kind:'original'|'retained'|'transition';source:number|null;loops:number}[];faceCount:number;pairCount:number;vertexCount:number;poleCount:number;components:number;supportEnclosures?:ExpressionDisplayReport;displayMesh?:ExpressionMeshReport;boundaryMesh?:BoundaryMeshReport;radiusCheck?:ExpressionRadiusReport;supportTangencyCheck?:SupportTangencyReport;neighborTangencyCheck?:NeighborTangencyReport;alignedChordCheck?:AlignedChordReport|null}
}
// Transport identity only. Rust recomputes every geometric qualification.
function identity(value:unknown):string {
 let nodes=0;const active=new Set<object>()
 const visit=(v:unknown,depth:number):unknown=>{
  if(++nodes>2_000_000||depth>128)throw new Error('Expression recipe exceeds transport limits')
  if(v===null)return ['null']
  if(typeof v==='string'||typeof v==='boolean')return [typeof v,v]
  if(typeof v==='number'&&Number.isFinite(v))return ['number',Object.is(v,-0)?'-0':String(v)]
  if(typeof v!=='object'||active.has(v))throw new Error('Invalid expression recipe')
  active.add(v)
  const out=Array.isArray(v)?['array',v.map(x=>visit(x,depth+1))]:['object',Object.keys(v).sort().map(k=>[k,visit((v as Record<string,unknown>)[k],depth+1)])]
  active.delete(v);return out
 }
 const result=JSON.stringify(visit(value,0))
 if(result.length>32*1024*1024)throw new Error('Expression recipe exceeds transport limits')
 return result
}
export function expressionShellExpectation(options:ExpressionShellOptions){
 if(options.alignedChordCheck&&!validAlignedChordRequest(options.alignedChordCheck))throw new Error('Bound native aligned chord coverage work')
 if(options.neighborTangencyCheck!==undefined&&!validNeighborTangencyRequest(options.neighborTangencyCheck))throw new Error('Bound native neighbor tangency work')
 if(options.supportTangencyCheck!==undefined&&!validSupportTangencyRequest(options.supportTangencyCheck))throw new Error('Bound native support tangency work')
 if(options.radiusCheck!==undefined&&!validExpressionRadiusRequest(options.radiusCheck))throw new Error('Bound native radius check work and laws')
 if(options.boundaryMeshLimits&&!validBoundaryMeshLimits(options.boundaryMeshLimits))throw new Error('Bound native boundary mesh work')
 if(options.displayLimits&&!validExpressionDisplayLimits(options.displayLimits))throw new Error('Bound expression display work')
 if(options.meshLimits&&!validExpressionMeshLimits(options.meshLimits))throw new Error('Bound expression mesh work')
 if(!Number.isSafeInteger(options.shellLimits.shellUses)||options.shellLimits.shellUses<0||options.shellLimits.shellUses>100000)throw new Error('Bound expression shell uses')
 return {alignedChordCheck:options.alignedChordCheck?structuredClone(options.alignedChordCheck):undefined,neighborTangencyCheck:options.neighborTangencyCheck?structuredClone(options.neighborTangencyCheck):undefined,supportTangencyCheck:options.supportTangencyCheck?structuredClone(options.supportTangencyCheck):undefined,radiusCheck:options.radiusCheck?structuredClone(options.radiusCheck):undefined,boundaryMeshLimits:options.boundaryMeshLimits?structuredClone(options.boundaryMeshLimits):undefined,requestIdentity:identity(options),proposalIdentity:identity(options.proposalDefinition),boundaryIdentity:identity(options.boundaryDefinition),maxFaces:options.shellLimits.shellUses,displayLimits:options.displayLimits?structuredClone(options.displayLimits):undefined,meshLimits:options.meshLimits?structuredClone(options.meshLimits):undefined}}
export function validExpressionShell(expected:ReturnType<typeof expressionShellExpectation>,value:unknown):value is ExpressionShellResult {
 try {
  if(!value||typeof value!=='object')return false
  const v=value as ExpressionShellResult
  if(v.version!==1||v.method!=='exact-expression-shell-replay'||typeof v.incidenceQualified!=='boolean'||v.embeddingQualified!==false||v.materialQualified!==false||typeof v.reason!=='string'||!v.reason.length
   ||identity(v.request)!==expected.requestIdentity||identity(v.proposalDefinition)!==expected.proposalIdentity||identity(v.boundaryDefinition)!==expected.boundaryIdentity)return false
  if(!v.incidenceQualified)return v.shell===null
  const s=v.shell;const count=(n:unknown)=>Number.isSafeInteger(n)&&(n as number)>=0
  if(!s||!Array.isArray(s.faces)||!count(s.faceCount)||s.faceCount<1||s.faceCount>expected.maxFaces||s.faces.length!==s.faceCount||!count(s.pairCount)||!count(s.vertexCount)||!count(s.poleCount)||!count(s.components)||s.components<1||s.components>s.faceCount||s.pairCount>expected.maxFaces||s.vertexCount>2*expected.maxFaces||s.poleCount>expected.maxFaces)return false
  if(s.faces.reduce((total,f)=>total+(count(f.loops)?f.loops:Infinity),0)>expected.maxFaces)return false
  if(expected.displayLimits?!validExpressionDisplay(expected.displayLimits,s.faceCount,s.supportEnclosures):s.supportEnclosures!==undefined)return false
  if(expected.meshLimits?!validExpressionMesh(expected.meshLimits,s.faceCount,s.displayMesh):s.displayMesh!==undefined)return false
  if(expected.boundaryMeshLimits?!validBoundaryMesh(expected.boundaryMeshLimits,s.faceCount,s.pairCount,s.boundaryMesh):s.boundaryMesh!==undefined)return false
  if(expected.alignedChordCheck?(s.alignedChordCheck!==null&&!validAlignedChordReport(expected.alignedChordCheck,s.faceCount,s.alignedChordCheck)):s.alignedChordCheck!==undefined)return false
  if(expected.neighborTangencyCheck?!validNeighborTangencyReport(expected.neighborTangencyCheck,s.neighborTangencyCheck):s.neighborTangencyCheck!==undefined)return false
  if(expected.supportTangencyCheck?!validSupportTangencyReport(expected.supportTangencyCheck,s.supportTangencyCheck):s.supportTangencyCheck!==undefined)return false
  if(expected.radiusCheck?!validExpressionRadiusReport(expected.radiusCheck,s.radiusCheck):s.radiusCheck!==undefined)return false
  return s.faces.every((f,i)=>f.id===i&&['original','retained','transition'].includes(f.kind)&&count(f.loops)&&f.loops>0&&(f.kind==='transition'?f.source===null:count(f.source)))
 }catch{return false}
}
export function restoreExpressionShell(options:ExpressionShellOptions):ExpressionShellResult {
 const expected=expressionShellExpectation(options)
 const result=callGeometryRust<ExpressionShellResult>('cad_expression_shell_restore',options)
 if(!validExpressionShell(expected,result))throw new Error('Invalid expression shell replay response')
 return result
}
export {identity as expressionRecipeIdentity}
