import {callGeometryRust} from './geometry/kernel'
/** Original native recipes only; display intervals never authorize geometry. */
export interface SourceBodyDefinition {
 version:1
 shell:{version:1;regions:unknown[];pairs:{uses:[number[],number[]];edge:unknown}[];poles:unknown[]}
}
export interface SourceBodyOptions {
 definition:SourceBodyDefinition
 endpointSpans:number
 limits:{
  shell:{exactWork:number;driverCells:number;regions:{region:{pairs:number;regionCells:number;domainCells:number;agreementCells:number};searchCells:number;pointChecks:number;mappingCells:number;driverCells:number;membershipCells:number;controls:number;windingCells:number;steps:number}}
  embedding:{toleranceUv:number;corners:number;spans:number;linearCells:number;exactWork:number;driverCells:number;pairs:{pairs:number;cells:number;domainCells:number;cellsPerPair:number;domainCellsPerPair:number}}
  volume:{axis:number;origin:number;absoluteError:number;toleranceUv:number;cells:number;spans:number;domainCells:number}
 }
}
type Interval=[number,number]
export interface SourceBodyEdge {
 index:number;definition:unknown;parameterBounds:[Interval,Interval];endpointBoxes:[[Interval,Interval,Interval],[Interval,Interval,Interval]]
 vertices:[number,number];uses:[number[],number[]]
}
export interface SourceBodyResult {
 admitted:boolean;sourceBody:SourceBodyDefinition|null;edges:SourceBodyEdge[]
 volume?:Interval;reverseOrientation?:boolean;faceCount?:number;poleCount?:number
 diagnostics:{reason:string;incidence:unknown;embedding:unknown;volume:unknown}
}
// Canonical JSON compares transport definitions independently of object key order.
// Rust alone recomputes root ownership, contacts and volume admission.
function key(value:unknown):string {
 const seen=new Set<object>();let nodes=0
 const visit=(v:unknown,depth:number):unknown=>{
  if(++nodes>2_000_000||depth>128)throw new Error('Source definition exceeds transport limits')
  if(v===null)return ['null']
  if(typeof v==='number'){
   if(!Number.isFinite(v))throw new Error('Nonfinite source value')
   return ['number',Object.is(v,-0)?'-0':String(v)]
  }
  if(typeof v==='string'||typeof v==='boolean')return [typeof v,v]
  if(typeof v!=='object'||seen.has(v))throw new Error('Invalid source JSON value')
  seen.add(v)
  const out=Array.isArray(v)?['array',v.map(x=>visit(x,depth+1))]
   :['object',Object.keys(v).sort().map(k=>[k,visit((v as Record<string,unknown>)[k],depth+1)])]
  seen.delete(v);return out
 }
 const text=JSON.stringify(visit(value,0))
 if(text.length>32*1024*1024)throw new Error('Source Body definition exceeds transport limits')
 return text
}
const integer=(n:unknown,max:number)=>typeof n==='number'&&Number.isSafeInteger(n)&&n>=0&&n<=max
const interval=(v:unknown):v is Interval=>Array.isArray(v)&&v.length===2&&v.every(n=>typeof n==='number'&&Number.isFinite(n))&&v[0]<=v[1]
export function sourceBodyExpectation(options:SourceBodyOptions) {
 const d=options.definition,s=d?.shell
 if(d?.version!==1||s?.version!==1||!Array.isArray(s.regions)||s.regions.length<2||s.regions.length>4096
  ||!Array.isArray(s.pairs)||s.pairs.length>524288||!Array.isArray(s.poles)||s.poles.length>1000000
  ||!integer(options.endpointSpans,100000)||options.endpointSpans<1)throw new Error('Invalid source Body request')
 const edges=s.pairs.map(pair=>{
  if(!pair||!Array.isArray(pair.uses)||pair.uses.length!==2||!pair.uses.every(a=>Array.isArray(a)&&a.length===3&&a.every(n=>integer(n,1000000))&&a[0]<s.regions.length))throw new Error('Invalid source edge address')
  return {definition:key(pair.edge),uses:key(pair.uses)}
 })
 return {definition:key({version:1,shell:s}),faces:s.regions.length,poles:s.poles.length,edges,
  absoluteError:options.limits.volume.absoluteError}
}
export function validSourceBody(e:ReturnType<typeof sourceBodyExpectation>,value:unknown):value is SourceBodyResult {
 try {
  const r=value as SourceBodyResult
  if(!r||typeof r.admitted!=='boolean'||!Array.isArray(r.edges)||typeof r.diagnostics?.reason!=='string')return false
  if(!r.admitted)return r.sourceBody===null&&r.edges.length===0&&r.diagnostics.reason.startsWith('source-')&&r.diagnostics.reason!=='source-volume-and-orientation-qualified'
  if(r.diagnostics.reason!=='source-volume-and-orientation-qualified'||!r.sourceBody||key(r.sourceBody)!==e.definition
   ||r.faceCount!==e.faces||r.poleCount!==e.poles||typeof r.reverseOrientation!=='boolean'
   ||!interval(r.volume)||r.volume[0]<=0||!Number.isFinite(e.absoluteError)||e.absoluteError<=0||r.volume[1]-r.volume[0]>e.absoluteError
   ||r.edges.length!==e.edges.length)return false
  return r.edges.every((edge,i)=>edge.index===i&&key(edge.definition)===e.edges[i]!.definition&&key(edge.uses)===e.edges[i]!.uses
   &&Array.isArray(edge.vertices)&&edge.vertices.length===2&&edge.vertices.every(n=>integer(n,2*e.edges.length+e.poles))
   &&Array.isArray(edge.parameterBounds)&&edge.parameterBounds.length===2&&edge.parameterBounds.every(interval)
   &&Array.isArray(edge.endpointBoxes)&&edge.endpointBoxes.length===2&&edge.endpointBoxes.every(box=>Array.isArray(box)&&box.length===3&&box.every(interval)))
 }catch{return false}
}
export function restoreSourceBody(options:SourceBodyOptions):SourceBodyResult {
 return callGeometryRust('cad_source_body_restore',options)
}
