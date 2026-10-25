// Transport validation; native Rust computes geometry and deviation bounds.
export interface ExpressionMeshLimits {
 maxDeviationMm:number;cells:number;controlsPerEnclosure:number;spans:number
 toleranceUv:number;domainCells:number;domainCellsPerCell:number;maxDepth:number
}
type Domain=[[number,number],[number,number]]
type Point=[number,number,number]
export interface ExpressionMeshReport {
 version:1;splitCells:number;leafCount:number
 watertightQualified:false;complete:boolean;cells:number;spans:number;domainCells:number;maxDeviationUpperMm:number
 trianglesPerTile:[[0,1,2],[0,2,3]]
 tiles:{face:number;domain:Domain;corners:[Point,Point,Point,Point];deviationUpperMm:number}[]
 pending:{face:number;domain:Domain;bounds:[[number,number],[number,number],[number,number]]|null;reason:string}[]
 outside:{face:number;domain:Domain}[]
}
const count=(n:unknown):n is number=>Number.isSafeInteger(n)&&(n as number)>=0
const finite=(n:unknown):n is number=>typeof n==='number'&&Number.isFinite(n)
const interval=(v:unknown)=>Array.isArray(v)&&v.length===2&&v.every(finite)&&v[0]<=v[1]
const domain=(v:unknown)=>Array.isArray(v)&&v.length===2&&v.every(interval)
export function validExpressionMeshLimits(l:ExpressionMeshLimits):boolean {
 return !!l&&finite(l.maxDeviationMm)&&l.maxDeviationMm>0&&finite(l.toleranceUv)&&l.toleranceUv>0
  &&count(l.cells)&&l.cells<=100000&&count(l.controlsPerEnclosure)&&l.controlsPerEnclosure<=1000000
  &&count(l.spans)&&l.spans<=100000&&count(l.domainCells)&&l.domainCells<=100000
  &&count(l.domainCellsPerCell)&&l.domainCellsPerCell<=100000&&count(l.maxDepth)&&l.maxDepth<=32
}
const reasons=new Set(['mesh-work-limit','mesh-support-unproven','mesh-trim-unproven','mesh-deviation-unproven','mesh-vertex-unproven'])
export function validExpressionMesh(l:ExpressionMeshLimits,faces:number,value:unknown):value is ExpressionMeshReport {
 if(!validExpressionMeshLimits(l)||!value||typeof value!=='object')return false
 const v=value as ExpressionMeshReport
 if(v.version!==1||v.watertightQualified!==false||typeof v.complete!=='boolean'||!Array.isArray(v.tiles)||!Array.isArray(v.pending)||!Array.isArray(v.outside)
  ||v.complete!==(v.pending.length===0)||JSON.stringify(v.trianglesPerTile)!=='[[0,1,2],[0,2,3]]'
  ||!finite(v.maxDeviationUpperMm)||v.maxDeviationUpperMm<0||v.maxDeviationUpperMm>l.maxDeviationMm)return false
 for(const key of ['cells','spans','domainCells'] as const)if(!count(v[key])||v[key]>l[key])return false
 if(!count(v.splitCells)||v.splitCells>v.cells||!count(v.leafCount)||v.leafCount!==faces+v.splitCells
  ||v.leafCount!==v.tiles.length+v.pending.length+v.outside.length
  ||v.cells-v.splitCells-v.tiles.length-v.outside.length<0
  ||v.cells-v.splitCells-v.tiles.length-v.outside.length>v.pending.length)return false
 if(v.tiles.length+v.outside.length>v.cells||v.tiles.length+v.outside.length+v.pending.length>v.cells+faces)return false
 const coverage=new Set<number>(),seen=new Set<string>()
 function cell(c:{face:number;domain:Domain}){
  if(!c||!count(c.face)||c.face>=faces||!domain(c.domain))return false
  const key=JSON.stringify([c.face,c.domain]);if(seen.has(key))return false
  seen.add(key);coverage.add(c.face);return true
 }
 let maximum=0
 for(const t of v.tiles){
  if(!cell(t)||!finite(t.deviationUpperMm)||t.deviationUpperMm<=0||t.deviationUpperMm>l.maxDeviationMm
   ||!Array.isArray(t.corners)||t.corners.length!==4||!t.corners.every(p=>Array.isArray(p)&&p.length===3&&p.every(finite)))return false
  maximum=Math.max(maximum,t.deviationUpperMm)
 }
 for(const p of v.pending)if(!cell(p)||!reasons.has(p.reason)||(p.bounds!==null&&(!Array.isArray(p.bounds)||p.bounds.length!==3||!p.bounds.every(interval))))return false
 for(const c of v.outside)if(!cell(c))return false
 return coverage.size===faces&&maximum===v.maxDeviationUpperMm
}
