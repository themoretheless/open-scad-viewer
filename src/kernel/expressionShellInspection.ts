import {callGeometryRust} from './geometry/kernel'
import {expressionShellExpectation,expressionRecipeIdentity,validExpressionShell,type ExpressionShellOptions,type ExpressionShellResult} from './expressionShell'
export interface ExpressionInspectionOptions extends ExpressionShellOptions {
 inspectionLimits:{
  projectedRegion:{toleranceUv:number;exactWork:number;driverCells:number}
  contacts:{cells:number;cellsPerPair:number;controlsPerEnclosure:number;spans:number;domainCells:number;domainCellsPerPair:number;exactWork:number;driverCells:number;toleranceUv:number}
  linearCells:number;linearCellsPerPair:number;refinementCellsPerPair:number
  volume:{axis:number;origin:number;absoluteError:number;toleranceUv:number;controlsPerEnclosure:number;cells:number;spans:number;domainCells:number;domainCellsPerRectangle:number}
 }
}
type Interval=[number,number]
type Product=[ [Interval,Interval], [Interval,Interval] ]
export interface ExpressionContacts {
 version:1;kind:'expression-shell-contact-audit';pendingPairs:number
 cells:number;spans:number;domainCells:number;exactWork:number;driverCells:number;linearCells:number
 faces:{index:number;kind:string;sourceFace?:number;retainedSide?:number}[]
 pendingByReason:Record<string,number>;pendingByFaceKinds:Record<string,number>
 pairs:{faces:[number,number];proof:string;reason:string;sharedEdges:number[];pendingProducts:Product[];outsideProducts:Product[];separatedProducts:Product[]}[]
}
export interface ExpressionInspection {
 embeddingQualified:boolean;volumeQualified:boolean;reason:string;contacts?:ExpressionContacts
 volume?:{complete:boolean;signedBounds:Interval|null;positiveBounds:Interval|null;cells:number;spans:number;domainCells:number;uncertainFace:number|null;uncertainDomain:[Interval,Interval]|null;reason:string}
}
export interface ExpressionInspectionResult extends Omit<ExpressionShellResult,'method'|'request'|'embeddingQualified'|'shell'> {
 method:'exact-expression-shell-inspection';request:ExpressionInspectionOptions;embeddingQualified:boolean
 shell:(NonNullable<ExpressionShellResult['shell']>&{inspection:ExpressionInspection})|null
}
export function expressionInspectionExpectation(options:ExpressionInspectionOptions){
 const {inspectionLimits,...base}=options
 return {base:expressionShellExpectation(base),requestIdentity:expressionRecipeIdentity(options),limits:structuredClone(inspectionLimits)}
}
const proofs=new Set(['pending','excluded','planar','fiber','vertex-or-interior','hull-separated','pole-contact','boundary-contact','profile-hull-separated','profile-boundary-contact','profile-paired-contact','profile-ruled-contact','profile-vertex-contact','profile-vertex-excluded','transition-coordinate-separated','transition-xy-bounds-separated','transition-xy-hull-separated','transition-xy-material-plane-separated','transition-plane','transition-ruled','transition-section'])
const count=(v:unknown):v is number=>Number.isSafeInteger(v)&&(v as number)>=0
const interval=(v:unknown):v is Interval=>Array.isArray(v)&&v.length===2&&v.every(x=>typeof x==='number'&&Number.isFinite(x))&&v[0]<=v[1]
const rectangle=(v:unknown)=>Array.isArray(v)&&v.length===2&&v.every(interval)
const products=(v:unknown)=>Array.isArray(v)&&v.every(p=>Array.isArray(p)&&p.length===2&&p.every(rectangle))
export function validExpressionInspection(expected:ReturnType<typeof expressionInspectionExpectation>,value:unknown):value is ExpressionInspectionResult {
 try{
  if(!value||typeof value!=='object')return false
  const v=value as ExpressionInspectionResult
  if(v.method!=='exact-expression-shell-inspection'||typeof v.embeddingQualified!=='boolean'||expressionRecipeIdentity(v.request)!==expected.requestIdentity)return false
  const {inspectionLimits:_,...base}=v.request
  if(!validExpressionShell(expected.base,{...v,method:'exact-expression-shell-replay',request:base,embeddingQualified:false}))return false
  if(!v.shell)return !v.embeddingQualified
  const s=v.shell,i=s.inspection,l=expected.limits
  if(expected.base.alignedChordCheck&&(v.embeddingQualified?s.alignedChordCheck===null:s.alignedChordCheck!==null))return false
  if(!i||typeof i.embeddingQualified!=='boolean'||i.embeddingQualified!==v.embeddingQualified||typeof i.volumeQualified!=='boolean'||typeof i.reason!=='string'||!i.reason)return false
  if(!i.contacts)return !i.embeddingQualified&&!i.volumeQualified&&i.volume===undefined
  const c=i.contacts
  if(c.version!==1||c.kind!=='expression-shell-contact-audit'||!count(c.pendingPairs)||!Array.isArray(c.faces)||c.faces.length!==s.faceCount||!Array.isArray(c.pairs)||c.pairs.length!==s.faceCount*(s.faceCount-1)/2)return false
  for(const [key,max] of [['cells',l.contacts.cells],['spans',l.contacts.spans],['domainCells',l.contacts.domainCells],['exactWork',l.contacts.exactWork],['driverCells',l.contacts.driverCells],['linearCells',l.linearCells]] as const)if(!count(c[key])||c[key]>max)return false
  if(!c.faces.every((f,n)=>f.index===n&&f.kind===s.faces[n].kind&&(f.kind==='original'?f.sourceFace===s.faces[n].source:f.kind==='retained'?f.retainedSide===s.faces[n].source:true)))return false
  const seen=new Set<string>();let pending=0
  for(const p of c.pairs){
   if(!Array.isArray(p.faces)||p.faces.length!==2||!p.faces.every(count)||p.faces[0]>=p.faces[1]||p.faces[1]>=s.faceCount||!proofs.has(p.proof)||typeof p.reason!=='string'||!p.reason||!Array.isArray(p.sharedEdges)||!p.sharedEdges.every(n=>count(n)&&n<s.pairCount)||!products(p.pendingProducts)||!products(p.outsideProducts)||!products(p.separatedProducts))return false
   const key=p.faces.join(':');if(seen.has(key))return false;seen.add(key);if(p.proof==='pending')pending++
  }
  if(c.pendingPairs!==pending||(i.embeddingQualified&&pending!==0))return false
  for(const histogram of [c.pendingByReason,c.pendingByFaceKinds])if(!histogram||Array.isArray(histogram)||typeof histogram!=='object'||Object.values(histogram).some(n=>!count(n))||Object.values(histogram).reduce((sum,n)=>sum+n,0)!==pending)return false
  if(!i.volume)return !i.volumeQualified&&!i.embeddingQualified
  const volume=i.volume
  if(!i.embeddingQualified||typeof volume.complete!=='boolean'||typeof volume.reason!=='string'||!volume.reason)return false
  for(const key of ['cells','spans','domainCells'] as const)if(!count(volume[key])||volume[key]>l.volume[key])return false
  if(volume.signedBounds!==null&&!interval(volume.signedBounds))return false
  if(volume.positiveBounds!==null&&!interval(volume.positiveBounds))return false
  if(volume.uncertainFace!==null&&(!count(volume.uncertainFace)||volume.uncertainFace>=s.faceCount))return false
  if(volume.uncertainDomain!==null&&!rectangle(volume.uncertainDomain))return false
  if(i.volumeQualified){
   const signed=volume.signedBounds,positive=volume.positiveBounds
   if(!volume.complete||!signed||!positive||positive[0]<=0||positive[1]-positive[0]>l.volume.absoluteError||volume.uncertainFace!==null||volume.uncertainDomain!==null)return false
   const oriented=signed[1]<0?[-signed[1],-signed[0]]:signed
   if(oriented[0]!==positive[0]||oriented[1]!==positive[1])return false
  }else if(volume.positiveBounds!==null)return false
  return true
 }catch{return false}
}
export function inspectExpressionShell(options:ExpressionInspectionOptions):ExpressionInspectionResult {
 const expected=expressionInspectionExpectation(options)
 const result=callGeometryRust<ExpressionInspectionResult>('cad_expression_shell_inspect',options)
 if(!validExpressionInspection(expected,result))throw new Error('Invalid expression shell inspection response')
 return result
}
