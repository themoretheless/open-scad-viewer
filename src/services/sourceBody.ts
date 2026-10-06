import {callGeometryRust} from './geometry/kernel'
/** Original native recipes only; display intervals never authorize geometry. */
export interface SourceBodyDefinition {
 version:1
 inverseShear?:{axes:[number,number];coefficient:number}
 shell:{version:1;regions:unknown[];pairs:{uses:[number[],number[]];edge:unknown}[];poles:unknown[]}
}
export interface SourceStepOptions {
 toleranceMm:number;trimWork:number
 limits:{rootChecks:number;mappingCells:number;replayMappingPerUse:number;exactWork:number;driverCells:number;spans:number;endpoints:number}
}
export type SourceStepResult = {prepared:false;request:SourceStepOptions;text:null;reason:'source-step-preparation-unproven'}
 | {prepared:true;request:SourceStepOptions;text:string;endpointErrorUpper:number;vertices:number;edges:number;faces:number}
export interface SourceBodyOptions {
 stepExchange?:SourceStepOptions
 definition:SourceBodyDefinition
 /** With edge display, bounds total original carrier spans over both ends. */
 endpointSpans:number
 displaySegments?:number
 faceDisplay?:{divisions:number;toleranceUv:number;domainCellsPerFace:number}
 limits:{
  shell:{exactWork:number;driverCells:number;regions:{region:{pairs:number;regionCells:number;domainCells:number;agreementCells:number};searchCells:number;pointChecks:number;mappingCells:number;driverCells:number;membershipCells:number;controls:number;windingCells:number;steps:number}}
  embedding:{toleranceUv:number;corners:number;spans:number;linearCells:number;exactWork:number;driverCells:number;pairs:{pairs:number;cells:number;domainCells:number;cellsPerPair:number;domainCellsPerPair:number}}
  volume:{axis:number;origin:number;absoluteError:number;toleranceUv:number;cells:number;spans:number;domainCells:number}
 }
}
type Interval=[number,number]
export interface SourceBodyEdge {
 index:number;definition:unknown;parameterBounds:[Interval,Interval];endpointBoxes:[[Interval,Interval,Interval],[Interval,Interval,Interval]]
 displaySegments?:[Interval,[[number,number,number],[number,number,number]],[Interval,Interval,Interval]][]|null
 vertices:[number,number];uses:[number[],number[]]
}
export interface SourceDisplayFace {
 index:number;tiles:{uv:[Interval,Interval];corners:[number,number,number][]}[]
 unresolved:[Interval,Interval][];unresolvedBoxes:[Interval,Interval,Interval][];outside:number;domainCells:number
}
export interface SourceBodyResult {
 admitted:boolean;sourceBody:SourceBodyDefinition|null;edges:SourceBodyEdge[]
 stepExchange?:SourceStepResult|null
 displayFaces?:SourceDisplayFace[]|null
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
 const step=options.stepExchange
 if(step){
  const limits=step.limits
  if(!Number.isFinite(step.toleranceMm)||step.toleranceMm<=0||!integer(step.trimWork,10_000_000)||step.trimWork<1
   ||!limits||!integer(limits.exactWork,100_000_000)||limits.exactWork<1||!integer(limits.driverCells,100000)
   ||!['rootChecks','mappingCells','replayMappingPerUse','spans','endpoints'].every(k=>integer(limits[k as keyof typeof limits],100000)&&limits[k as keyof typeof limits]>=1))throw new Error('Invalid source STEP exchange limits')
 }
 const inverse=d.inverseShear
 if(inverse&&(!Array.isArray(inverse.axes)||inverse.axes.length!==2||!inverse.axes.every(n=>integer(n,2))
  ||inverse.axes[0]===inverse.axes[1]||!Number.isFinite(inverse.coefficient)))throw new Error('Invalid inverse shear recipe')
 const segments=options.displaySegments??0
 if(!integer(segments,4096)||segments*s.pairs.length>65536)throw new Error('Source display exceeds transport limits')
 const face=options.faceDisplay
 if(face&&(!integer(face.divisions,64)||face.divisions<1||!integer(face.domainCellsPerFace,100000)||face.domainCellsPerFace<1
  ||!Number.isFinite(face.toleranceUv)||face.toleranceUv<=0||s.regions.length*face.divisions**2>65536
  ||s.regions.length*face.domainCellsPerFace>1000000))throw new Error('Source face display exceeds transport limits')
 const edges=s.pairs.map(pair=>{
  if(!pair||!Array.isArray(pair.uses)||pair.uses.length!==2||!pair.uses.every(a=>Array.isArray(a)&&a.length===3&&a.every(n=>integer(n,1000000))&&a[0]<s.regions.length))throw new Error('Invalid source edge address')
  return {definition:key(pair.edge),uses:key(pair.uses)}
 })
 return {stepExchange:step?key(step):null,stepTolerance:step?.toleranceMm,definition:key({version:1,shell:s,...(inverse?{inverseShear:{axes:inverse.axes,coefficient:inverse.coefficient}}:{})}),faces:s.regions.length,poles:s.poles.length,edges,
  absoluteError:options.limits.volume.absoluteError,segments,faceDisplay:face?{...face}:null}
}
export function validSourceBody(e:ReturnType<typeof sourceBodyExpectation>,value:unknown):value is SourceBodyResult {
 try {
  const r=value as SourceBodyResult
  if(!r||typeof r.admitted!=='boolean'||!Array.isArray(r.edges)||typeof r.diagnostics?.reason!=='string')return false
  if(!r.admitted)return r.stepExchange==null&&r.sourceBody===null&&r.edges.length===0&&r.diagnostics.reason.startsWith('source-')&&r.diagnostics.reason!=='source-volume-and-orientation-qualified'
  if(r.diagnostics.reason!=='source-volume-and-orientation-qualified'||!r.sourceBody||key(r.sourceBody)!==e.definition
   ||r.faceCount!==e.faces||r.poleCount!==e.poles||typeof r.reverseOrientation!=='boolean'
   ||!interval(r.volume)||r.volume[0]<=0||!Number.isFinite(e.absoluteError)||e.absoluteError<=0||r.volume[1]-r.volume[0]>e.absoluteError
   ||r.edges.length!==e.edges.length)return false
  const step=r.stepExchange
  if(e.stepExchange){
   if(!step||key(step.request)!==e.stepExchange||typeof step.prepared!=='boolean')return false
   if(step.prepared){
    if(typeof step.text!=='string'||step.text.length>32*1024*1024||!step.text.startsWith('ISO-10303-21;\n')||!step.text.endsWith('END-ISO-10303-21;\n')
     ||!Number.isFinite(step.endpointErrorUpper)||step.endpointErrorUpper<0||step.endpointErrorUpper>e.stepTolerance!
     ||step.edges!==e.edges.length+e.poles||step.faces!==e.faces||!integer(step.vertices,2*e.edges.length+e.poles)||step.vertices<1)return false
   }else if(step.text!==null||step.reason!=='source-step-preparation-unproven')return false
  }else if(step!=null)return false
  const rectangle=(v:unknown)=>Array.isArray(v)&&v.length===2&&v.every(interval)
  if(e.faceDisplay){
   const option=e.faceDisplay
   if(!Array.isArray(r.displayFaces)||r.displayFaces.length!==e.faces||!r.displayFaces.every((face,i)=>face.index===i
    &&Array.isArray(face.tiles)&&Array.isArray(face.unresolved)&&integer(face.outside,option.divisions**2)
    &&integer(face.domainCells,option.domainCellsPerFace)&&face.tiles.length+face.unresolved.length+face.outside===option.divisions**2
    &&Array.isArray(face.unresolvedBoxes)&&face.unresolvedBoxes.length===face.unresolved.length&&face.unresolvedBoxes.every(box=>Array.isArray(box)&&box.length===3&&box.every(interval))
    &&face.unresolved.every(rectangle)&&face.tiles.every(tile=>rectangle(tile.uv)&&Array.isArray(tile.corners)&&tile.corners.length===4
     &&tile.corners.every(p=>Array.isArray(p)&&p.length===3&&p.every(Number.isFinite)))))return false
  }else if(r.displayFaces!=null)return false
  const displayValid=(edge:SourceBodyEdge)=>{
   if(e.segments===0)return edge.displaySegments==null
   const list=edge.displaySegments
   if(!Array.isArray(list)||list.length!==e.segments)return false
   return list.every((segment,i)=>Array.isArray(segment)&&segment.length===3&&interval(segment[0])&&segment[0][0]<segment[0][1]
    &&Array.isArray(segment[1])&&segment[1].length===2&&segment[1].every(p=>Array.isArray(p)&&p.length===3&&p.every(Number.isFinite))
    &&Array.isArray(segment[2])&&segment[2].length===3&&segment[2].every(interval)
    &&(i===0?segment[0][0]===edge.parameterBounds[0][0]:segment[0][0]===list[i-1]![0][1])
    &&(i===0||segment[1][0].every((n,a)=>n===list[i-1]![1][1][a]))
    &&(i!==list.length-1||segment[0][1]===edge.parameterBounds[1][1]))
  }
  const edgesValid=r.edges.every((edge,i)=>edge.index===i&&key(edge.definition)===e.edges[i]!.definition&&key(edge.uses)===e.edges[i]!.uses
   &&Array.isArray(edge.vertices)&&edge.vertices.length===2&&edge.vertices.every(n=>integer(n,2*e.edges.length+e.poles))
   &&Array.isArray(edge.parameterBounds)&&edge.parameterBounds.length===2&&edge.parameterBounds.every(interval)
   &&Array.isArray(edge.endpointBoxes)&&edge.endpointBoxes.length===2&&edge.endpointBoxes.every(box=>Array.isArray(box)&&box.length===3&&box.every(interval))&&displayValid(edge))
  if(!edgesValid)return false
  const anchors=new Map<number,number[]>()
  for(const edge of r.edges){
   if(e.segments===0)continue
   const segments=edge.displaySegments!
   for(const segment of segments){
    if(!segment[1].every(point=>point.every((n,a)=>n>=segment[2][a]![0]&&n<=segment[2][a]![1])))return false
   }
   for(const end of [0,1] as const){
    const point=segments[end===0?0:segments.length-1]![1][end],box=edge.endpointBoxes[end],id=edge.vertices[end]
    if(!point.every((n,a)=>n>=box[a]![0]&&n<=box[a]![1]))return false
    const previous=anchors.get(id)
    if(previous&&!point.every((n,a)=>n===previous[a]))return false
    anchors.set(id,point)
   }
  }
  return true
 }catch{return false}
}
export function restoreSourceBody(options:SourceBodyOptions):SourceBodyResult {
 return callGeometryRust('cad_source_body_restore',options)
}
