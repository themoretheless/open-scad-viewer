// Transport checks only. Rust owns trim classification and position bounds.
export interface ExpressionDisplayLimits {
 divisions:number;cells:number;controlsPerEnclosure:number;spans:number
 toleranceUv:number;domainCells:number;domainCellsPerCell:number
}
type Interval=[number,number]
export interface ExpressionDisplayReport {
 trimQualified:false;enclosed:number;pending:number;spans:number;domainCells:number
 inside:number;outside:number;unresolvedTrim:number
 cells:{face:number;domain:[Interval,Interval];bounds:[Interval,Interval,Interval]|null;location:'inside'|'outside'|'unresolved'}[]
}
const count=(n:unknown):n is number=>Number.isSafeInteger(n)&&(n as number)>=0
const interval=(v:unknown):v is Interval=>Array.isArray(v)&&v.length===2&&v.every(x=>typeof x==='number'&&Number.isFinite(x))&&v[0]<=v[1]
export function validExpressionDisplayLimits(l:ExpressionDisplayLimits):boolean {
 return !!l&&count(l.divisions)&&l.divisions>=1&&l.divisions<=64
  &&count(l.cells)&&l.cells<=100000&&count(l.controlsPerEnclosure)&&l.controlsPerEnclosure<=1000000
  &&count(l.spans)&&l.spans<=100000&&count(l.domainCells)&&l.domainCells<=100000
  &&count(l.domainCellsPerCell)&&l.domainCellsPerCell<=100000
  &&Number.isFinite(l.toleranceUv)&&l.toleranceUv>0
}
export function validExpressionDisplay(l:ExpressionDisplayLimits,faces:number,value:unknown):value is ExpressionDisplayReport {
 if(!value||typeof value!=='object'||!validExpressionDisplayLimits(l))return false
 const v=value as ExpressionDisplayReport
 if(v.trimQualified!==false||!Array.isArray(v.cells)||v.cells.length<faces||v.cells.length>l.cells+faces
  ||!count(v.spans)||v.spans>l.spans||!count(v.domainCells)||v.domainCells>l.domainCells
  ||![v.enclosed,v.pending,v.inside,v.outside,v.unresolvedTrim].every(count))return false
 const seen=new Set<string>(),coverage=new Map<number,number>()
 let enclosed=0,inside=0,outside=0,unresolved=0
 for(const c of v.cells){
  if(!c||!count(c.face)||c.face>=faces||!Array.isArray(c.domain)||c.domain.length!==2||!c.domain.every(interval))return false
  if(c.bounds!==null&&(!Array.isArray(c.bounds)||c.bounds.length!==3||!c.bounds.every(interval)))return false
  const key=JSON.stringify([c.face,c.domain]);if(seen.has(key))return false;seen.add(key)
  coverage.set(c.face,(coverage.get(c.face)??0)+1)
  if(c.bounds!==null)enclosed++
  if(c.location==='inside')inside++;else if(c.location==='outside')outside++;else if(c.location==='unresolved')unresolved++;else return false
 }
 return coverage.size===faces&&[...coverage.values()].every(n=>n<=l.divisions*l.divisions)
  &&v.enclosed===enclosed&&v.pending===v.cells.length-enclosed&&v.inside===inside&&v.outside===outside&&v.unresolvedTrim===unresolved
}
