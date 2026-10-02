import type {NurbsBrep} from './geometry/brep'
import type {VolumeValidityLimits} from './solidDistance'
import {callGeometryRust} from './geometry/kernel'
type Interval=[number,number]
export interface NormalAudit {maxSineSquared:number;maxSpans:number}
interface NormalEndpoint {face:number;uv:[Interval,Interval];aligned:boolean|null;sineSquaredInterval:Interval|null;normalComponents:[Interval,Interval,Interval]|null;spans:number;reason:string}
interface NormalEvidence {aligned:boolean|null;spans:number;endpoints:[NormalEndpoint|null,NormalEndpoint|null]}
export interface MaterialOptions {normalAudit?:NormalAudit;model:NurbsBrep;origin:[number,number,number];direction:[number,number,number];toleranceUv:number;limits:{validity:VolumeValidityLimits;pointCells:number;pointDomainCells:number;segmentCells:number;segmentDomainCells:number}}
interface Crossing {face:number;uv:[Interval,Interval];parameter:Interval}
interface Unresolved {face:number;uv:[Interval,Interval];reason:string}
interface Boundary {contacts:Crossing[];unresolved:Unresolved[];cells:number;domainCells:number;boundaryFree?:boolean}
interface Seed {inside:boolean|null;cells:number;domainCells:number;attempts:Array<{inside:boolean|null;crossings:Crossing[];unresolved:Unresolved[];cells:number;domainCells:number}>}
interface Validity {proven:boolean;boundaryProven:boolean;exactAgreement:boolean;exactJoins:boolean;trimValid:boolean;selfIntersectionAbsent:boolean;nestingRolesConsistent:boolean|null;orientations:Array<{shell:number;expectedOutward:boolean;outward:boolean|null}>}
interface Base {sourceModel:NurbsBrep;origin:number[];direction:number[];parameterInterval:Interval;toleranceUv:number;limits:MaterialOptions['limits'];validity:Validity;seed:Seed|null;proven:boolean;reason:string}
export interface MaterialSegment extends Base {method:'continuous-material-segment';scope:'strict-interior-authored-parametric-segment';segment:Boundary|null}
export interface MaterialChord extends Base {method:'continuous-material-chord';scope:'material-between-original-transverse-boundary-roots';normalAlignment:'not-qualified'|'angular-tolerance'|'oblique'|'unresolved';normalAudit?:NormalAudit|null;normalEvidence?:NormalEvidence|null;minimumWallThickness:'not-qualified';boundary:Boundary;pointEnclosures:[Interval[],Interval[]]|null;lengthIntervalMm:Interval|null}
export type MaterialResult=MaterialSegment|MaterialChord
export interface MaterialOverlay {line:[[number,number,number],[number,number,number]];marks:Array<{point:[number,number,number];face:number;unresolved:boolean}>;proven:boolean}
const snapshot=(v:unknown)=>JSON.stringify(v,(_,x)=>x&&typeof x==='object'&&!Array.isArray(x)?Object.fromEntries(Object.keys(x).sort().map(k=>[k,x[k]])):x)
const interval=(x:unknown):x is Interval=>Array.isArray(x)&&x.length===2&&x.every(Number.isFinite)&&x[0]<=x[1]
const work=(x:unknown,max:number)=>typeof x==='number'&&Number.isSafeInteger(x)&&x>=0&&x<=max
const bool=(x:unknown)=>x===null||typeof x==='boolean'
export function materialExpectation(options:MaterialOptions,mode:'segment'|'chord') {
 return {mode,normalAudit:structuredClone(options.normalAudit??null),source:snapshot(options.model),origin:snapshot(options.origin),direction:snapshot(options.direction),toleranceUv:options.toleranceUv,limits:structuredClone(options.limits),
  shells:options.model.shells.map((_,i)=>!options.model.bodies.some(b=>b.innerShells.includes(i))),
  normalSpans:options.model.faces.map(({surface:s})=>[s.knotsU.slice(s.degreeU,s.controlPoints.length+1),s.knotsV.slice(s.degreeV,s.controlPoints[0].length+1)]),
  domains:options.model.faces.map(({surface:s})=>[[s.knotsU[s.degreeU],s.knotsU[s.controlPoints.length]],[s.knotsV[s.degreeV],s.knotsV[s.controlPoints[0].length]]] as [Interval,Interval])}
}
export function validMaterial(e:ReturnType<typeof materialExpectation>,value:unknown):value is MaterialResult {
 const r=value as MaterialResult
 if(!r||r.method!==`continuous-material-${e.mode}`||r.scope!==(e.mode==='segment'?'strict-interior-authored-parametric-segment':'material-between-original-transverse-boundary-roots')||snapshot(r.sourceModel)!==e.source||snapshot(r.origin)!==e.origin||snapshot(r.direction)!==e.direction||snapshot(r.parameterInterval)!=='[0,1]'||r.toleranceUv!==e.toleranceUv||snapshot(r.limits)!==snapshot(e.limits)||typeof r.proven!=='boolean')return false
 const v=r.validity
 if(!v||!['proven','boundaryProven','exactAgreement','exactJoins','trimValid','selfIntersectionAbsent'].every(k=>typeof v[k as keyof Validity]==='boolean')||!bool(v.nestingRolesConsistent)||!Array.isArray(v.orientations)||v.orientations.length!==e.shells.length)return false
 if(v.boundaryProven&&!(v.exactAgreement&&v.exactJoins&&v.trimValid&&v.selfIntersectionAbsent))return false
 let oriented=true
 for(const [i,o] of v.orientations.entries()){
  if(!o||o.shell!==i||o.expectedOutward!==e.shells[i]||!bool(o.outward)||((!v.boundaryProven||v.nestingRolesConsistent!==true)&&o.outward!==null))return false
  oriented&&=o.outward===o.expectedOutward
 }
 if(!v.boundaryProven&&v.nestingRolesConsistent!==null)return false
 if(v.proven!==(v.boundaryProven&&v.nestingRolesConsistent===true&&oriented))return false
 const uv=(c:Crossing|Unresolved)=>c&&work(c.face,e.domains.length-1)&&Array.isArray(c.uv)&&c.uv.length===2&&c.uv.every((x,k)=>interval(x)&&x[0]>=e.domains[c.face][k][0]&&x[1]<=e.domains[c.face][k][1])
 const crossing=(c:Crossing,finite:boolean)=>uv(c)&&interval(c.parameter)&&c.parameter[0]>0&&(!finite||c.parameter[1]<1)
 const unresolved=(c:Unresolved)=>uv(c)&&['work-limit','domain-work-limit','root-not-isolated','trim-boundary','endpoint-band','origin-band','overlapping-root-intervals'].includes(c.reason)
 const seed=r.seed
 if(seed!==null){
  if(!seed||!bool(seed.inside)||!work(seed.cells,e.limits.pointCells)||!work(seed.domainCells,e.limits.pointDomainCells)||!Array.isArray(seed.attempts)||seed.attempts.length<1||seed.attempts.length>3)return false
  let cells=0,domains=0
  for(const [i,a] of seed.attempts.entries()){
   if(!a||!bool(a.inside)||!work(a.cells,e.limits.pointCells)||!work(a.domainCells,e.limits.pointDomainCells)||!Array.isArray(a.crossings)||!a.crossings.every(c=>crossing(c,false))||!Array.isArray(a.unresolved)||!a.unresolved.every(unresolved))return false
   if(a.inside!==(a.unresolved.length?null:a.crossings.length%2===1)||i<seed.attempts.length-1&&a.inside!==null)return false
   for(let j=1;j<a.crossings.length;j++)if(a.crossings[j-1].parameter[0]>a.crossings[j].parameter[0]||a.inside!==null&&a.crossings[j-1].parameter[1]>=a.crossings[j].parameter[0])return false
   cells+=a.cells;domains+=a.domainCells
  }
  if(cells!==seed.cells||domains!==seed.domainCells||seed.inside!==seed.attempts.at(-1)!.inside)return false
 }
 const b=r.method==='continuous-material-chord'?r.boundary:r.segment
 if(b!==null&&(!b||!work(b.cells,e.limits.segmentCells)||!work(b.domainCells,e.limits.segmentDomainCells)||!Array.isArray(b.contacts)||!b.contacts.every(c=>crossing(c,true))||!Array.isArray(b.unresolved)||!b.unresolved.every(unresolved)))return false
 if(r.method==='continuous-material-segment'){
  if(b!==null&&b.boundaryFree!==(b.contacts.length===0&&b.unresolved.length===0))return false
  if(!v.proven)return !r.proven&&r.reason==='volume-unproven'&&seed===null&&b===null
  if(!seed)return false
  if(seed.inside!==true)return !r.proven&&b===null&&r.reason===(seed.inside===false?'seed-outside':'seed-unresolved')
  return b!==null&&r.proven===b.boundaryFree&&r.reason===(b.boundaryFree?'interior-segment':b.contacts.length?'boundary-contact':'segment-unresolved')
 }
 if(r.minimumWallThickness!=='not-qualified'||b===null||!validNormals(e,r))return false
 let reason='volume-unproven'
 if(v.proven){
  if(!seed)return false
  reason=seed.inside!==false?(seed.inside===true?'seed-inside':'seed-unresolved'):b.unresolved.length?'boundary-unresolved':b.contacts.length!==2?'requires-two-crossings':b.contacts[0].parameter[1]>=b.contacts[1].parameter[0]?'overlapping-root-intervals':'material-chord'
 }else if(seed!==null)return false
 if(r.reason!==reason||r.proven!==(reason==='material-chord'))return false
 if(!r.proven)return r.lengthIntervalMm===null&&r.pointEnclosures===null
 const d=r.lengthIntervalMm,p=r.pointEnclosures
 if(!interval(d)||d[0]<0||!Array.isArray(p)||p.length!==2||!p.every(x=>Array.isArray(x)&&x.length===3&&x.every(interval)))return false
 const lower=Math.hypot(...p[0].map((x,k)=>Math.max(0,x[0]-p[1][k][1],p[1][k][0]-x[1])))
 const upper=Math.hypot(...p[0].map((x,k)=>Math.max(Math.abs(x[0]-p[1][k][1]),Math.abs(x[1]-p[1][k][0]))))
 const slack=Number.EPSILON*Math.max(1,upper,...p.flat(2).map(Math.abs))*32
 return d[0]<=lower+slack&&d[1]>=upper-slack
}
export const inspectMaterialSegment=(options:MaterialOptions):MaterialSegment=>callGeometryRust('cad_material_segment',options)
export const inspectMaterialChord=(options:MaterialOptions):MaterialChord=>callGeometryRust('cad_material_chord',options)

function validNormals(e:ReturnType<typeof materialExpectation>,r:MaterialChord):boolean {
 const config=e.normalAudit
 if(snapshot(r.normalAudit??null)!==snapshot(config))return false
 const n=r.normalEvidence
 if(config===null)return r.normalAlignment==='not-qualified'&&n==null
 if(!Number.isFinite(config.maxSineSquared)||config.maxSineSquared<0||config.maxSineSquared>=1||!Number.isSafeInteger(config.maxSpans)||config.maxSpans<1||config.maxSpans>100000)return false
 if(!n||!bool(n.aligned)||!work(n.spans,config.maxSpans)||!Array.isArray(n.endpoints)||n.endpoints.length!==2)return false
 let spans=0
 for(const [i,a] of n.endpoints.entries()){
  if(a===null){if(r.proven&&spans<config.maxSpans)return false;continue}
  const c=r.boundary.contacts[i]
  if(!r.proven||!c||a.face!==c.face||snapshot(a.uv)!==snapshot(c.uv)||!bool(a.aligned)||!work(a.spans,config.maxSpans-spans))return false
  const available=config.maxSpans-spans
  const required=e.normalSpans[a.face].reduce((total,knots,axis)=>total*knots.slice(0,-1).filter((lo,j)=>lo<knots[j+1]&&a.uv[axis][0]<=knots[j+1]&&a.uv[axis][1]>=lo).length,1)
  if(required<1||a.spans!==Math.min(required,available)||(a.reason==='span-limit')!==(required>available))return false
  spans+=a.spans
  const d=a.sineSquaredInterval,h=a.normalComponents
  if(h!==null&&(!Array.isArray(h)||h.length!==3||!h.every(interval)))return false
  if(d!==null&&(!interval(d)||d[0]<0||d[1]>1||h===null||h.every(x=>x[0]<=0&&x[1]>=0)))return false
  if(a.reason==='angular-tolerance'){if(a.aligned!==true||d===null||d[1]>config.maxSineSquared)return false}
  else if(a.reason==='oblique'){if(a.aligned!==false||d===null||d[0]<=config.maxSineSquared)return false}
  else if(a.reason==='angular-unresolved'){if(a.aligned!==null||d===null||d[0]>config.maxSineSquared||d[1]<=config.maxSineSquared)return false}
  else if(a.reason==='normal-unresolved'){if(a.aligned!==null||d!==null||h===null)return false}
  else if(a.reason==='span-limit'){if(a.aligned!==null||d!==null||h!==null)return false}
  else return false
 }
 const aligned=n.endpoints.some(a=>a?.aligned===false)?false:n.endpoints.every(a=>a?.aligned===true)?true:null
 return spans===n.spans&&n.aligned===aligned&&r.normalAlignment===(aligned===true?'angular-tolerance':aligned===false?'oblique':'unresolved')
}
